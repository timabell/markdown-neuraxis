package co.rustworkshop.markdownneuraxis

import android.content.Context
import android.net.Uri
import android.os.Bundle
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
import androidx.compose.material3.*
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
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield
import uniffi.markdown_neuraxis_ffi.DocumentHandle
import uniffi.markdown_neuraxis_ffi.RenderBlockDto

private const val TAG = "MarkdownNeuraxis"
private const val PREFS_NAME = "markdown_neuraxis_prefs"
private const val KEY_NOTES_URI = "notes_uri"
private const val FILE_SCAN_BATCH_SIZE = 20

/**
 * Represents a node in the file tree - either a folder or a file
 */
sealed class FileTreeNode {
    abstract val name: String
    abstract val depth: Int

    data class Folder(
        override val name: String,
        override val depth: Int,
        val documentFile: DocumentFile,
        val children: MutableList<FileTreeNode> = mutableListOf(),
        var isExpanded: Boolean = false
    ) : FileTreeNode()

    data class File(
        override val name: String,
        override val depth: Int,
        val documentFile: DocumentFile
    ) : FileTreeNode()
}

/**
 * Manages the file tree structure for progressive loading
 */
class FileTree {
    private val root: MutableList<FileTreeNode> = mutableListOf()
    private val folderMap: MutableMap<String, FileTreeNode.Folder> = mutableMapOf()

    fun addFile(file: DocumentFile, pathSegments: List<String>) {
        if (pathSegments.isEmpty()) return

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
                    documentFile = file, // Will be replaced when we have actual folder DocumentFile
                    isExpanded = false // Start collapsed
                )
                folderMap[pathKey] = newFolder
                currentChildren.add(newFolder)
                currentChildren.sortBy { node ->
                    when (node) {
                        is FileTreeNode.Folder -> "0_${node.name.lowercase()}"
                        is FileTreeNode.File -> "1_${node.name.lowercase()}"
                    }
                }
                currentChildren = newFolder.children
            }
            currentDepth++
        }

        // Add the file
        val fileName = pathSegments.last()
        val fileNode = FileTreeNode.File(
            name = fileName,
            depth = currentDepth,
            documentFile = file
        )
        currentChildren.add(fileNode)
        currentChildren.sortBy { node ->
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

    when {
        notesUri == null -> {
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
    onFileSelected: (DocumentFile) -> Unit,
    onChangeFolder: () -> Unit
) {
    val context = LocalContext.current
    var discoveryState by remember { mutableStateOf(FileDiscoveryState()) }
    // Force recomposition when tree structure changes
    var treeVersion by remember { mutableIntStateOf(0) }

    // Progressive file scanning using LaunchedEffect
    LaunchedEffect(notesUri) {
        val tree = FileTree()
        discoveryState = FileDiscoveryState(tree = tree, isScanning = true)
        try {
            scanMarkdownFilesProgressively(context, notesUri) { batch ->
                for (fileWithPath in batch) {
                    tree.addFile(fileWithPath.file, fileWithPath.pathSegments)
                }
                discoveryState = discoveryState.copy(
                    fileCount = discoveryState.fileCount + batch.size
                )
                treeVersion++
            }
            discoveryState = discoveryState.copy(isScanning = false)
        } catch (e: Exception) {
            Log.e(TAG, "Error scanning files", e)
            discoveryState = discoveryState.copy(
                isScanning = false,
                error = e.message ?: "Unknown error"
            )
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

                Box(modifier = Modifier.padding(padding)) {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize()
                    ) {
                        items(flattenedNodes.size) { index ->
                            val node = flattenedNodes[index]
                            FileTreeNodeItem(
                                node = node,
                                onFileSelected = onFileSelected,
                                onFolderToggle = {
                                    discoveryState.tree.toggleFolder(it)
                                    treeVersion++
                                }
                            )
                        }
                    }

                    // Status toast overlay in top-right
                    if (discoveryState.isScanning) {
                        StatusToast(
                            message = "Scanning... ${discoveryState.fileCount} files",
                            showProgress = true,
                            modifier = Modifier.align(Alignment.TopEnd)
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun FileTreeNodeItem(
    node: FileTreeNode,
    onFileSelected: (DocumentFile) -> Unit,
    onFolderToggle: (FileTreeNode.Folder) -> Unit
) {
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
                    .clickable { onFileSelected(node.documentFile) }
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
 * Data class to hold file with its path segments for tree building
 */
data class FileWithPath(
    val file: DocumentFile,
    val pathSegments: List<String>
)

/**
 * Progressively scan for markdown files, calling onBatch for each batch of files found.
 * This allows the UI to update as files are discovered rather than waiting for the full scan.
 */
private suspend fun scanMarkdownFilesProgressively(
    context: Context,
    folderUri: Uri,
    onBatch: (List<FileWithPath>) -> Unit
) {
    withContext(Dispatchers.IO) {
        val folder = DocumentFile.fromTreeUri(context, folderUri) ?: return@withContext
        val batch = mutableListOf<FileWithPath>()
        // Wrap callback to ensure UI updates happen on main thread
        val mainThreadCallback: suspend (List<FileWithPath>) -> Unit = { files ->
            withContext(Dispatchers.Main) {
                onBatch(files)
            }
        }
        scanFolderRecursively(folder, emptyList(), batch, mainThreadCallback)
        // Emit any remaining files in the final batch
        if (batch.isNotEmpty()) {
            mainThreadCallback(batch.toList())
        }
    }
}

/**
 * Recursively scan a folder for markdown files, emitting batches as they're found.
 */
private suspend fun scanFolderRecursively(
    folder: DocumentFile,
    pathPrefix: List<String>,
    batch: MutableList<FileWithPath>,
    onBatch: suspend (List<FileWithPath>) -> Unit
) {
    for (file in folder.listFiles().orEmpty()) {
        if (file.isDirectory) {
            val folderName = file.name ?: continue
            scanFolderRecursively(file, pathPrefix + folderName, batch, onBatch)
        } else if (file.name?.endsWith(".md") == true) {
            val fileName = file.name ?: continue
            batch.add(FileWithPath(file, pathPrefix + fileName))
            if (batch.size >= FILE_SCAN_BATCH_SIZE) {
                onBatch(batch.toList())
                batch.clear()
                yield() // Allow UI thread to process updates
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
