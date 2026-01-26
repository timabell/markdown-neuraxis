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
import uniffi.markdown_neuraxis_ffi.DocumentHandle
import uniffi.markdown_neuraxis_ffi.RenderBlockDto

private const val TAG = "MarkdownNeuraxis"
private const val PREFS_NAME = "markdown_neuraxis_prefs"
private const val KEY_NOTES_URI = "notes_uri"

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
    // Note: Only lists files in the root folder, not recursively in subfolders
    val files = remember(notesUri) {
        listMarkdownFiles(context, notesUri)
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
        if (files.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center
            ) {
                Text("No markdown files found in root folder")
            }
        } else {
            LazyColumn(
                modifier = Modifier.padding(padding)
            ) {
                items(files) { file ->
                    ListItem(
                        headlineContent = { Text(file.name ?: "Unknown") },
                        modifier = Modifier.clickable { onFileSelected(file) }
                    )
                    HorizontalDivider()
                }
            }
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

private fun listMarkdownFiles(context: Context, folderUri: Uri): List<DocumentFile> {
    val folder = DocumentFile.fromTreeUri(context, folderUri) ?: return emptyList()
    return folder.listFiles()
        .filter { it.isFile && it.name?.endsWith(".md") == true }
        .sortedBy { it.name }
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
