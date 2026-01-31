package co.rustworkshop.markdownneuraxis

import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.FolderOpen
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.material3.*
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.ui.theme.MarkdownNeuraxisTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield
import uniffi.markdown_neuraxis_ffi.DocumentHandle
import uniffi.markdown_neuraxis_ffi.RenderBlockDto

private const val TAG = "MarkdownNeuraxis"
private const val PREFS_NAME = "markdown_neuraxis_prefs"
private const val KEY_NOTES_URI = "notes_uri"
private const val FILE_SCAN_BATCH_SIZE = 20
private const val FILE_CACHE_NAME = "file_cache.txt"

/**
 * Represents a node in the file tree - either a folder or a file.
 * Stores relative path for cache persistence; DocumentFile resolved on demand.
 */
sealed class FileTreeNode {
    abstract val name: String
    abstract val depth: Int
    abstract val relativePath: String

    data class Folder(
        override val name: String,
        override val depth: Int,
        override val relativePath: String,
        val children: MutableList<FileTreeNode> = mutableListOf(),
        var isExpanded: Boolean = false
    ) : FileTreeNode()

    data class File(
        override val name: String,
        override val depth: Int,
        override val relativePath: String,
        var documentFile: DocumentFile? = null // Resolved on demand
    ) : FileTreeNode()
}

/**
 * Manages the file tree structure for progressive loading
 */
class FileTree {
    private val root: MutableList<FileTreeNode> = mutableListOf()
    private val folderMap: MutableMap<String, FileTreeNode.Folder> = mutableMapOf()
    private val fileMap: MutableMap<String, FileTreeNode.File> = mutableMapOf()

    /**
     * Add a file from path segments only (for cache loading).
     * DocumentFile will be null until resolved on demand.
     */
    fun addFilePath(pathSegments: List<String>) {
        addFileInternal(pathSegments, null)
    }

    /**
     * Add a file with its DocumentFile (from scanning).
     */
    fun addFile(file: DocumentFile, pathSegments: List<String>) {
        addFileInternal(pathSegments, file)
    }

    private fun addFileInternal(pathSegments: List<String>, documentFile: DocumentFile?) {
        if (pathSegments.isEmpty()) return

        val relativePath = pathSegments.joinToString("/")

        // Skip if already exists (cache may have it, scan will update)
        if (fileMap.containsKey(relativePath)) {
            // Update DocumentFile if we now have one
            if (documentFile != null) {
                fileMap[relativePath]?.documentFile = documentFile
            }
            return
        }

        var currentChildren = root
        var currentDepth = 0

        // Navigate/create folders for all but the last segment
        for (i in 0 until pathSegments.size - 1) {
            val folderName = pathSegments[i]
            val pathKey = pathSegments.subList(0, i + 1).joinToString("/")

            val existingFolder = folderMap[pathKey]
            if (existingFolder != null) {
                currentChildren = existingFolder.children
            } else {
                val newFolder = FileTreeNode.Folder(
                    name = folderName,
                    depth = currentDepth,
                    relativePath = pathKey,
                    isExpanded = false // Start collapsed
                )
                folderMap[pathKey] = newFolder
                currentChildren.add(newFolder)
                sortChildren(currentChildren)
                currentChildren = newFolder.children
            }
            currentDepth++
        }

        // Add the file
        val fileName = pathSegments.last()
        val fileNode = FileTreeNode.File(
            name = fileName,
            depth = currentDepth,
            relativePath = relativePath,
            documentFile = documentFile
        )
        fileMap[relativePath] = fileNode
        currentChildren.add(fileNode)
        sortChildren(currentChildren)
    }

    private fun sortChildren(children: MutableList<FileTreeNode>) {
        children.sortBy { node ->
            when (node) {
                is FileTreeNode.Folder -> "0_${node.name.lowercase()}"
                is FileTreeNode.File -> "1_${node.name.lowercase()}"
            }
        }
    }

    fun getRootNodes(): List<FileTreeNode> = root.toList()

    fun toggleFolder(folder: FileTreeNode.Folder) {
        folder.isExpanded = !folder.isExpanded
    }

    /**
     * Get all file relative paths for cache saving.
     */
    fun getAllFilePaths(): List<String> = fileMap.keys.toList().sorted()

    fun getFileCount(): Int = fileMap.size

    /**
     * Remove files that are no longer present (not in scannedPaths).
     * Returns count of removed files.
     */
    fun removeStaleFiles(scannedPaths: Set<String>): Int {
        val stalePaths = fileMap.keys.filter { it !in scannedPaths }
        for (path in stalePaths) {
            removeFile(path)
        }
        return stalePaths.size
    }

    private fun removeFile(relativePath: String) {
        val fileNode = fileMap.remove(relativePath) ?: return

        // Find and remove from parent's children
        val segments = relativePath.split("/")
        if (segments.size == 1) {
            // Root level file
            root.removeIf { it is FileTreeNode.File && it.relativePath == relativePath }
        } else {
            // Nested file - find parent folder
            val parentPath = segments.dropLast(1).joinToString("/")
            val parentFolder = folderMap[parentPath]
            parentFolder?.children?.removeIf { it is FileTreeNode.File && it.relativePath == relativePath }

            // Clean up empty folders
            cleanupEmptyFolders()
        }
    }

    private fun cleanupEmptyFolders() {
        // Remove empty folders from deepest to shallowest
        val foldersToRemove = mutableListOf<String>()
        for ((path, folder) in folderMap) {
            if (folder.children.isEmpty()) {
                foldersToRemove.add(path)
            }
        }

        for (path in foldersToRemove.sortedByDescending { it.count { c -> c == '/' } }) {
            val folder = folderMap.remove(path) ?: continue
            val segments = path.split("/")
            if (segments.size == 1) {
                root.removeIf { it is FileTreeNode.Folder && it.relativePath == path }
            } else {
                val parentPath = segments.dropLast(1).joinToString("/")
                val parentFolder = folderMap[parentPath]
                parentFolder?.children?.removeIf { it is FileTreeNode.Folder && it.relativePath == path }
            }
        }

        // Recursively clean if we removed folders that made parent folders empty
        if (foldersToRemove.isNotEmpty()) {
            cleanupEmptyFolders()
        }
    }

    fun getFlattenedList(): List<FileTreeNode> {
        val result = mutableListOf<FileTreeNode>()
        fun flatten(nodes: List<FileTreeNode>) {
            for (node in nodes) {
                result.add(node)
                if (node is FileTreeNode.Folder && node.isExpanded) {
                    flatten(node.children)
                }
            }
        }
        flatten(root)
        return result
    }
}

/**
 * State for progressive file discovery
 */
data class FileDiscoveryState(
    val tree: FileTree = FileTree(),
    val fileCount: Int = 0,
    val isScanning: Boolean = false,
    val error: String? = null
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MarkdownNeuraxisTheme {
                App()
            }
        }
    }
}

@Composable
fun App() {
    val context = LocalContext.current
    var notesUri by remember { mutableStateOf(getValidNotesUri(context)) }
    var selectedFile by remember { mutableStateOf<DocumentFile?>(null) }

    // Hoist discovery state to App level so it persists across navigation
    var discoveryState by remember { mutableStateOf(FileDiscoveryState()) }
    var treeVersion by remember { mutableIntStateOf(0) }
    var hasScannedThisSession by remember { mutableStateOf(false) }

    when {
        notesUri == null -> {
            // Reset state when folder changes
            discoveryState = FileDiscoveryState()
            hasScannedThisSession = false

            SetupScreen(
                onFolderSelected = { uri ->
                    saveNotesUri(context, uri)
                    notesUri = uri
                }
            )
        }
        selectedFile != null -> {
            FileViewScreen(
                file = selectedFile!!,
                onBack = { selectedFile = null }
            )
        }
        else -> {
            FileListScreen(
                notesUri = notesUri!!,
                discoveryState = discoveryState,
                onDiscoveryStateChange = { discoveryState = it },
                treeVersion = treeVersion,
                onTreeVersionIncrement = { treeVersion++ },
                hasScannedThisSession = hasScannedThisSession,
                onHasScannedChange = { hasScannedThisSession = it },
                onFileSelected = { file -> selectedFile = file },
                onChangeFolder = {
                    notesUri = null
                }
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SetupScreen(onFolderSelected: (Uri) -> Unit) {
    val context = LocalContext.current
    val folderPicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree()
    ) { uri ->
        uri?.let {
            // Take persistent permission
            val takeFlags = android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    android.content.Intent.FLAG_GRANT_WRITE_URI_PERMISSION
            context.contentResolver.takePersistableUriPermission(it, takeFlags)
            onFolderSelected(it)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Markdown Neuraxis") })
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = "Welcome to Markdown Neuraxis",
                style = MaterialTheme.typography.headlineMedium
            )
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = "Select your notes folder to get started",
                style = MaterialTheme.typography.bodyLarge
            )
            Spacer(modifier = Modifier.height(32.dp))
            Button(onClick = { folderPicker.launch(null) }) {
                Icon(Icons.Default.FolderOpen, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Choose Folder")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FileListScreen(
    notesUri: Uri,
    discoveryState: FileDiscoveryState,
    onDiscoveryStateChange: (FileDiscoveryState) -> Unit,
    treeVersion: Int,
    onTreeVersionIncrement: () -> Unit,
    hasScannedThisSession: Boolean,
    onHasScannedChange: (Boolean) -> Unit,
    onFileSelected: (DocumentFile) -> Unit,
    onChangeFolder: () -> Unit
) {
    val context = LocalContext.current
    val coroutineScope = rememberCoroutineScope()
    // Local state for live scanning count (updates immediately, unlike hoisted state)
    var scanningFileCount by remember { mutableIntStateOf(0) }

    // Function to perform scan (used by initial load and pull-to-refresh)
    // Adds new files and removes files that no longer exist
    suspend fun performScan(isRefresh: Boolean) {
        val tree = if (isRefresh) discoveryState.tree else FileTree()
        val scannedPaths = mutableSetOf<String>()

        scanningFileCount = 0
        onDiscoveryStateChange(discoveryState.copy(tree = tree, isScanning = true))
        onTreeVersionIncrement()
        try {
            scanMarkdownFilesProgressively(context, notesUri) { batch ->
                for (fileWithPath in batch) {
                    val path = fileWithPath.pathSegments.joinToString("/")
                    scannedPaths.add(path)
                    if (fileWithPath.file != null) {
                        tree.addFile(fileWithPath.file, fileWithPath.pathSegments)
                    } else {
                        tree.addFilePath(fileWithPath.pathSegments)
                    }
                }
                // Update local count with scanned count for responsive UI
                scanningFileCount = scannedPaths.size
                // Also update hoisted state
                onDiscoveryStateChange(FileDiscoveryState(tree = tree, fileCount = tree.getFileCount(), isScanning = true))
                onTreeVersionIncrement()
            }

            // Remove files that no longer exist
            val removedCount = tree.removeStaleFiles(scannedPaths)
            if (removedCount > 0) {
                Log.d(TAG, "Removed $removedCount stale files")
                onTreeVersionIncrement()
            }

            saveFileCache(context, tree.getAllFilePaths())
            onDiscoveryStateChange(discoveryState.copy(tree = tree, fileCount = tree.getFileCount(), isScanning = false))
            onHasScannedChange(true)
        } catch (e: Exception) {
            Log.e(TAG, "Error scanning files", e)
            onDiscoveryStateChange(discoveryState.copy(
                isScanning = false,
                error = if (!isRefresh && discoveryState.fileCount == 0) e.message ?: "Unknown error" else null
            ))
        }
    }

    // Load cache only if tree is empty (first time entering this screen)
    LaunchedEffect(Unit) {
        // Skip if we already have data
        if (discoveryState.fileCount > 0) {
            Log.d(TAG, "Using existing tree with ${discoveryState.fileCount} files")
            return@LaunchedEffect
        }

        val tree = FileTree()

        // Load from cache instantly
        val cachedPaths = loadFileCache(context)
        if (cachedPaths.isNotEmpty()) {
            for (path in cachedPaths) {
                tree.addFilePath(path.split("/"))
            }
            onDiscoveryStateChange(FileDiscoveryState(tree = tree, fileCount = tree.getFileCount()))
            onTreeVersionIncrement()
            Log.d(TAG, "Loaded ${cachedPaths.size} files from cache")
        }

        // Only scan on first app startup (no cache) and never scanned this session
        if (cachedPaths.isEmpty() && !hasScannedThisSession) {
            performScan(isRefresh = false)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Notes") },
                actions = {
                    IconButton(onClick = onChangeFolder) {
                        Icon(Icons.Default.FolderOpen, contentDescription = "Change folder")
                    }
                }
            )
        }
    ) { padding ->
        when {
            discoveryState.error != null -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text("Error: ${discoveryState.error}")
                }
            }
            discoveryState.isScanning && discoveryState.fileCount == 0 -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        CircularProgressIndicator()
                        Spacer(modifier = Modifier.height(16.dp))
                        Text("Scanning for markdown files...")
                    }
                }
            }
            !discoveryState.isScanning && discoveryState.fileCount == 0 -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text("No markdown files found")
                }
            }
            else -> {
                // Use treeVersion to force recomposition when tree changes
                val flattenedNodes = remember(treeVersion) {
                    discoveryState.tree.getFlattenedList()
                }

                // Pull-to-refresh with spring effect
                val pullToRefreshState = rememberPullToRefreshState()
                val pullProgress = pullToRefreshState.distanceFraction
                // Track if scan was triggered - reset only when pull returns to idle
                var scanTriggered by remember { mutableStateOf(false) }
                if (pullProgress == 0f) {
                    scanTriggered = false
                }

                PullToRefreshBox(
                    isRefreshing = discoveryState.isScanning,
                    onRefresh = {
                        scanTriggered = true
                        coroutineScope.launch {
                            performScan(isRefresh = true)
                        }
                    },
                    state = pullToRefreshState,
                    modifier = Modifier.padding(padding),
                    indicator = {
                        // Show "Refresh" only while user is actively pulling (before scan triggers)
                        if (pullProgress > 0f && !scanTriggered) {
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .graphicsLayer {
                                        alpha = pullProgress.coerceIn(0f, 1f)
                                    },
                                contentAlignment = Alignment.Center
                            ) {
                                Text(
                                    text = "Refresh",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }
                ) {
                    Box(modifier = Modifier.fillMaxSize()) {
                        // Content moves down with pull (only before scan triggers)
                        val translation = if (scanTriggered || pullProgress == 0f) 0f else pullProgress * 60f
                        LazyColumn(
                            modifier = Modifier
                                .fillMaxSize()
                                .graphicsLayer {
                                    translationY = translation.dp.toPx()
                                }
                        ) {
                            items(flattenedNodes.size) { index ->
                                val node = flattenedNodes[index]
                                FileTreeNodeItem(
                                    node = node,
                                    notesUri = notesUri,
                                    onFileSelected = onFileSelected,
                                    onFolderToggle = {
                                        discoveryState.tree.toggleFolder(it)
                                        onTreeVersionIncrement()
                                    }
                                )
                            }
                        }

                        // Status toast overlay in top-right
                        if (discoveryState.isScanning) {
                            StatusToast(
                                message = "Scanning... $scanningFileCount files",
                                showProgress = true,
                                modifier = Modifier.align(Alignment.TopEnd)
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun FileTreeNodeItem(
    node: FileTreeNode,
    notesUri: Uri,
    onFileSelected: (DocumentFile) -> Unit,
    onFolderToggle: (FileTreeNode.Folder) -> Unit
) {
    val context = LocalContext.current
    val indentPadding = (node.depth * 16).dp

    when (node) {
        is FileTreeNode.Folder -> {
            ListItem(
                headlineContent = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = if (node.isExpanded) "▼" else "▶",
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.width(16.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text = node.name,
                            style = MaterialTheme.typography.bodyLarge.copy(
                                fontWeight = FontWeight.Medium
                            )
                        )
                    }
                },
                modifier = Modifier
                    .clickable { onFolderToggle(node) }
                    .padding(start = indentPadding)
            )
        }
        is FileTreeNode.File -> {
            ListItem(
                headlineContent = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Spacer(modifier = Modifier.width(20.dp))
                        Text(node.name)
                    }
                },
                modifier = Modifier
                    .clickable {
                        // Resolve DocumentFile on demand if not already cached
                        val docFile = node.documentFile
                            ?: resolveDocumentFile(context, notesUri, node.relativePath)
                        if (docFile != null) {
                            node.documentFile = docFile // Cache for next time
                            onFileSelected(docFile)
                        } else {
                            Log.e(TAG, "Could not resolve file: ${node.relativePath}")
                        }
                    }
                    .padding(start = indentPadding)
            )
        }
    }
    HorizontalDivider()
}

/**
 * Reusable status toast overlay for showing transient status messages.
 * Appears as a small pill in the corner without affecting layout.
 */
@Composable
fun StatusToast(
    message: String,
    modifier: Modifier = Modifier,
    showProgress: Boolean = false
) {
    Surface(
        modifier = modifier.padding(8.dp),
        shape = MaterialTheme.shapes.small,
        color = MaterialTheme.colorScheme.surfaceVariant,
        tonalElevation = 2.dp,
        shadowElevation = 4.dp
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            if (showProgress) {
                CircularProgressIndicator(
                    modifier = Modifier.size(14.dp),
                    strokeWidth = 2.dp
                )
                Spacer(modifier = Modifier.width(8.dp))
            }
            Text(
                text = message,
                style = MaterialTheme.typography.bodySmall
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FileViewScreen(
    file: DocumentFile,
    onBack: () -> Unit
) {
    // Handle Android back button
    BackHandler(onBack = onBack)

    val context = LocalContext.current
    val content = remember(file) {
        readFileContent(context, file)
    }
    val snapshot = remember(content) {
        content?.let { parseDocument(it) }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(file.name ?: "File") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        when {
            content == null -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text("Error reading file")
                }
            }
            snapshot == null -> {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding),
                    contentAlignment = Alignment.Center
                ) {
                    Text("Error parsing document")
                }
            }
            else -> {
                LazyColumn(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .padding(horizontal = 16.dp)
                ) {
                    items(snapshot) { block ->
                        RenderBlock(block)
                    }
                }
            }
        }
    }
}

@Composable
fun RenderBlock(block: RenderBlockDto) {
    val indent = (block.depth.toInt() * 16).dp

    when (block.kind) {
        "heading" -> {
            val style = when (block.headingLevel.toInt()) {
                1 -> MaterialTheme.typography.headlineLarge
                2 -> MaterialTheme.typography.headlineMedium
                3 -> MaterialTheme.typography.headlineSmall
                4 -> MaterialTheme.typography.titleLarge
                5 -> MaterialTheme.typography.titleMedium
                else -> MaterialTheme.typography.titleSmall
            }
            Text(
                text = block.content,
                style = style,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(vertical = 8.dp)
            )
        }
        "list_item" -> {
            Row(modifier = Modifier.padding(start = indent, top = 4.dp, bottom = 4.dp)) {
                Text(
                    text = block.listMarker ?: "-",
                    modifier = Modifier.width(24.dp)
                )
                Text(text = block.content)
            }
        }
        "paragraph" -> {
            Text(
                text = block.content,
                modifier = Modifier.padding(vertical = 4.dp)
            )
        }
        "code_fence" -> {
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant,
                shape = MaterialTheme.shapes.small,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 4.dp)
            ) {
                Text(
                    text = block.content,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(8.dp)
                )
            }
        }
        "block_quote" -> {
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 4.dp)
            ) {
                Text(
                    text = block.content,
                    style = MaterialTheme.typography.bodyMedium.copy(
                        fontWeight = FontWeight.Light
                    ),
                    modifier = Modifier.padding(start = 16.dp, top = 8.dp, bottom = 8.dp, end = 8.dp)
                )
            }
        }
        "thematic_break" -> {
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
        }
        else -> {
            Text(
                text = block.content,
                modifier = Modifier.padding(vertical = 4.dp)
            )
        }
    }
}

// ============ Helper Functions ============

/**
 * Data class to hold file path segments for tree building.
 * DocumentFile is no longer resolved during scanning for performance;
 * it is resolved on demand when the user clicks a file.
 */
data class FileWithPath(
    val file: DocumentFile?,
    val pathSegments: List<String>
)

/**
 * Progressively scan for markdown files, calling onBatch for each batch of files found.
 * Uses DocumentsContract + ContentResolver.query() directly instead of DocumentFile.listFiles()
 * for dramatically better performance (single cursor query per folder vs N+1 queries).
 */
private suspend fun scanMarkdownFilesProgressively(
    context: Context,
    folderUri: Uri,
    onBatch: (List<FileWithPath>) -> Unit
) {
    withContext(Dispatchers.IO) {
        val rootDocId = DocumentsContract.getTreeDocumentId(folderUri)
        val batch = mutableListOf<FileWithPath>()
        val mainThreadCallback: suspend (List<FileWithPath>) -> Unit = { files ->
            withContext(Dispatchers.Main) {
                onBatch(files)
            }
        }
        scanFolderRecursively(context, folderUri, rootDocId, emptyList(), batch, mainThreadCallback)
        if (batch.isNotEmpty()) {
            mainThreadCallback(batch.toList())
        }
    }
}

/**
 * Recursively scan a folder for markdown files using DocumentsContract for fast
 * cursor-based directory listing. Each folder is a single ContentResolver query
 * instead of creating individual DocumentFile objects per child.
 */
private suspend fun scanFolderRecursively(
    context: Context,
    treeUri: Uri,
    parentDocId: String,
    pathPrefix: List<String>,
    batch: MutableList<FileWithPath>,
    onBatch: suspend (List<FileWithPath>) -> Unit
) {
    val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
    val projection = arrayOf(
        DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        DocumentsContract.Document.COLUMN_MIME_TYPE
    )

    val cursor = context.contentResolver.query(childrenUri, projection, null, null, null)
    if (cursor == null) {
        Log.w(TAG, "ContentResolver query returned null for: $childrenUri")
        return
    }

    cursor.use {
        val idIndex = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
        val nameIndex = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        val mimeIndex = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE)

        while (it.moveToNext()) {
            val docId = it.getString(idIndex) ?: continue
            val name = it.getString(nameIndex) ?: continue
            val mimeType = it.getString(mimeIndex) ?: ""

            if (mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
                scanFolderRecursively(context, treeUri, docId, pathPrefix + name, batch, onBatch)
            } else if (name.endsWith(".md")) {
                batch.add(FileWithPath(null, pathPrefix + name))
                if (batch.size >= FILE_SCAN_BATCH_SIZE) {
                    onBatch(batch.toList())
                    batch.clear()
                    yield()
                }
            }
        }
    }
}

/**
 * Get the saved notes URI if it exists and the permission is still valid.
 * Returns null if no URI saved or permission was revoked.
 */
private fun getValidNotesUri(context: Context): Uri? {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    val uriString = prefs.getString(KEY_NOTES_URI, null) ?: return null
    val uri = Uri.parse(uriString)

    // Check if we still have permission for this URI
    val hasPermission = context.contentResolver.persistedUriPermissions.any {
        it.uri == uri && it.isReadPermission
    }

    if (!hasPermission) {
        Log.w(TAG, "Permission lost for saved URI: $uri")
        // Clear the invalid URI
        prefs.edit().remove(KEY_NOTES_URI).apply()
        return null
    }

    return uri
}

private fun saveNotesUri(context: Context, uri: Uri) {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    prefs.edit().putString(KEY_NOTES_URI, uri.toString()).apply()
}

/**
 * Load cached file paths from app storage.
 * Returns empty list if no cache exists.
 */
private fun loadFileCache(context: Context): List<String> {
    val cacheFile = java.io.File(context.filesDir, FILE_CACHE_NAME)
    return if (cacheFile.exists()) {
        try {
            cacheFile.readLines().filter { it.isNotBlank() }
        } catch (e: Exception) {
            Log.w(TAG, "Error loading file cache", e)
            emptyList()
        }
    } else {
        emptyList()
    }
}

/**
 * Save file paths to cache in app storage.
 */
private fun saveFileCache(context: Context, paths: List<String>) {
    val cacheFile = java.io.File(context.filesDir, FILE_CACHE_NAME)
    try {
        cacheFile.writeText(paths.joinToString("\n"))
        Log.d(TAG, "Saved ${paths.size} paths to cache")
    } catch (e: Exception) {
        Log.e(TAG, "Error saving file cache", e)
    }
}

/**
 * Resolve a relative path to a DocumentFile by navigating from the root folder.
 * Uses DocumentsContract queries for fast lookup instead of DocumentFile.findFile()
 * which does a full directory listing per path segment.
 */
private fun resolveDocumentFile(context: Context, notesUri: Uri, relativePath: String): DocumentFile? {
    val segments = relativePath.split("/")
    var currentDocId = DocumentsContract.getTreeDocumentId(notesUri)

    for (segment in segments) {
        val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(notesUri, currentDocId)
        val projection = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME
        )
        var foundId: String? = null
        context.contentResolver.query(childrenUri, projection, null, null, null)?.use { cursor ->
            val idIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val nameIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
            while (cursor.moveToNext()) {
                if (cursor.getString(nameIndex) == segment) {
                    foundId = cursor.getString(idIndex)
                    break
                }
            }
        }
        currentDocId = foundId ?: return null
    }

    val docUri = DocumentsContract.buildDocumentUriUsingTree(notesUri, currentDocId)
    return DocumentFile.fromSingleUri(context, docUri)
}

private fun readFileContent(context: Context, file: DocumentFile): String? {
    return try {
        context.contentResolver.openInputStream(file.uri)?.use { stream ->
            stream.bufferedReader().readText()
        }
    } catch (e: Exception) {
        Log.e(TAG, "Error reading file: ${file.uri}", e)
        null
    }
}

private fun parseDocument(content: String): List<RenderBlockDto>? {
    return try {
        val doc = DocumentHandle.fromString(content)
        doc.getSnapshot().blocks
    } catch (e: Exception) {
        Log.e(TAG, "Error parsing document", e)
        null
    }
}
