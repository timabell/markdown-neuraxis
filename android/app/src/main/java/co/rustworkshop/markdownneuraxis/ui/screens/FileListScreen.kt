package co.rustworkshop.markdownneuraxis.ui.screens

import android.net.Uri
import android.util.Log
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.FolderOpen
import androidx.compose.material3.*
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.*
import co.rustworkshop.markdownneuraxis.model.FileDiscoveryState
import co.rustworkshop.markdownneuraxis.model.FileTree
import co.rustworkshop.markdownneuraxis.ui.components.FileTreeNodeItem
import co.rustworkshop.markdownneuraxis.ui.components.StatusToast
import kotlinx.coroutines.launch

private const val TAG = "MarkdownNeuraxis"

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
    var scanningFileCount by remember { mutableIntStateOf(0) }

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
                scanningFileCount = scannedPaths.size
                onDiscoveryStateChange(FileDiscoveryState(tree = tree, fileCount = tree.getFileCount(), isScanning = true))
                onTreeVersionIncrement()
            }

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

    LaunchedEffect(Unit) {
        if (discoveryState.fileCount > 0) {
            Log.d(TAG, "Using existing tree with ${discoveryState.fileCount} files")
            return@LaunchedEffect
        }

        val tree = FileTree()

        val cachedPaths = loadFileCache(context)
        if (cachedPaths.isNotEmpty()) {
            for (path in cachedPaths) {
                tree.addFilePath(path.split("/"))
            }
            onDiscoveryStateChange(FileDiscoveryState(tree = tree, fileCount = tree.getFileCount()))
            onTreeVersionIncrement()
            Log.d(TAG, "Loaded ${cachedPaths.size} files from cache")
        }

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
                val flattenedNodes = remember(treeVersion) {
                    discoveryState.tree.getFlattenedList()
                }

                val pullToRefreshState = rememberPullToRefreshState()
                val pullProgress = pullToRefreshState.distanceFraction
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
