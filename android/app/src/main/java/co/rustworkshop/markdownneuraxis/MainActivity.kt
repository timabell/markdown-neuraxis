package co.rustworkshop.markdownneuraxis

import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.clearFileCache
import co.rustworkshop.markdownneuraxis.io.getValidNotesUri
import co.rustworkshop.markdownneuraxis.io.saveNotesUri
import co.rustworkshop.markdownneuraxis.model.FileDiscoveryState
import co.rustworkshop.markdownneuraxis.ui.components.AppBottomBar
import co.rustworkshop.markdownneuraxis.ui.components.AppDrawerContent
import co.rustworkshop.markdownneuraxis.ui.screens.FileListScreen
import co.rustworkshop.markdownneuraxis.ui.screens.FileViewScreen
import co.rustworkshop.markdownneuraxis.ui.screens.MissingFileScreen
import co.rustworkshop.markdownneuraxis.ui.screens.SetupScreen
import co.rustworkshop.markdownneuraxis.ui.theme.MarkdownNeuraxisTheme
import kotlinx.coroutines.launch

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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun App() {
    val context = LocalContext.current
    var notesUri by remember { mutableStateOf(getValidNotesUri(context)) }
    val fileStack = remember { mutableStateListOf<DocumentFile>() }
    var missingFileName by remember { mutableStateOf<String?>(null) }
    var previousUri by remember { mutableStateOf<Uri?>(null) }

    var discoveryState by remember { mutableStateOf(FileDiscoveryState()) }
    var treeVersion by remember { mutableIntStateOf(0) }
    var hasScannedThisSession by remember { mutableStateOf(false) }

    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
    val scope = rememberCoroutineScope()

    val isSetup = notesUri == null
    val hasMissing = missingFileName != null
    val hasFile = fileStack.isNotEmpty()

    BackHandler(enabled = drawerState.isOpen || hasFile || hasMissing) {
        when {
            drawerState.isOpen -> scope.launch { drawerState.close() }
            hasMissing -> missingFileName = null
            hasFile -> fileStack.removeAt(fileStack.lastIndex)
        }
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        gesturesEnabled = !isSetup,
        drawerContent = {
            AppDrawerContent(
                onChangeFolder = {
                    previousUri = notesUri
                    notesUri = null
                },
                onCloseDrawer = { scope.launch { drawerState.close() } }
            )
        }
    ) {
        Scaffold(
            topBar = {
                when {
                    isSetup -> TopAppBar(title = { Text("Markdown Neuraxis") })
                    hasMissing -> TopAppBar(
                        title = { Text(missingFileName!!) },
                        navigationIcon = {
                            IconButton(onClick = { missingFileName = null }) {
                                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                            }
                        }
                    )
                    hasFile -> TopAppBar(
                        title = { Text(fileStack.last().name ?: "File") },
                        navigationIcon = {
                            IconButton(onClick = { fileStack.removeAt(fileStack.lastIndex) }) {
                                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                            }
                        }
                    )
                    else -> TopAppBar(title = { Text("Notes") })
                }
            },
            bottomBar = {
                if (!isSetup) {
                    AppBottomBar(
                        onMenuClick = { scope.launch { drawerState.open() } },
                        onHomeClick = {
                            fileStack.clear()
                            missingFileName = null
                        }
                    )
                }
            }
        ) { padding ->
            when {
                isSetup -> {
                    SetupScreen(
                        onFolderSelected = { uri ->
                            previousUri = null
                            fileStack.clear()
                            missingFileName = null
                            discoveryState = FileDiscoveryState()
                            hasScannedThisSession = false
                            clearFileCache(context)
                            saveNotesUri(context, uri)
                            notesUri = uri
                        },
                        onCancel = previousUri?.let {
                            { notesUri = it; previousUri = null }
                        },
                        modifier = Modifier.padding(padding)
                    )
                }
                hasMissing -> {
                    MissingFileScreen(
                        fileName = missingFileName!!,
                        modifier = Modifier.padding(padding)
                    )
                }
                hasFile -> {
                    FileViewScreen(
                        file = fileStack.last(),
                        fileTree = discoveryState.tree,
                        notesUri = notesUri!!,
                        onNavigateToFile = { file -> fileStack.add(file) },
                        onMissingFile = { name -> missingFileName = name },
                        modifier = Modifier.padding(padding)
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
                        onFileSelected = { file -> fileStack.add(file) },
                        modifier = Modifier.padding(padding)
                    )
                }
            }
        }
    }
}
