package co.rustworkshop.markdownneuraxis

import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.clearFileCache
import co.rustworkshop.markdownneuraxis.io.getValidNotesUri
import co.rustworkshop.markdownneuraxis.io.saveNotesUri
import co.rustworkshop.markdownneuraxis.model.FileDiscoveryState
import co.rustworkshop.markdownneuraxis.ui.screens.FileListScreen
import co.rustworkshop.markdownneuraxis.ui.screens.FileViewScreen
import co.rustworkshop.markdownneuraxis.ui.screens.SetupScreen
import co.rustworkshop.markdownneuraxis.ui.theme.MarkdownNeuraxisTheme

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
    var previousUri by remember { mutableStateOf<Uri?>(null) }

    var discoveryState by remember { mutableStateOf(FileDiscoveryState()) }
    var treeVersion by remember { mutableIntStateOf(0) }
    var hasScannedThisSession by remember { mutableStateOf(false) }

    when {
        notesUri == null -> {
            discoveryState = FileDiscoveryState()
            hasScannedThisSession = false

            SetupScreen(
                onFolderSelected = { uri ->
                    previousUri = null
                    clearFileCache(context)
                    saveNotesUri(context, uri)
                    notesUri = uri
                },
                onCancel = previousUri?.let {
                    { notesUri = it; previousUri = null }
                }
            )
        }
        selectedFile != null -> {
            FileViewScreen(
                file = selectedFile!!,
                fileTree = discoveryState.tree,
                notesUri = notesUri!!,
                onBack = { selectedFile = null },
                onNavigateToFile = { file -> selectedFile = file }
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
                    previousUri = notesUri
                    notesUri = null
                }
            )
        }
    }
}
