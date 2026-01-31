package co.rustworkshop.markdownneuraxis.ui.screens

import android.util.Log
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.readFileContent
import uniffi.markdown_neuraxis_ffi.DocumentHandle
import uniffi.markdown_neuraxis_ffi.RenderBlockDto

private const val TAG = "MarkdownNeuraxis"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FileViewScreen(
    file: DocumentFile,
    onBack: () -> Unit
) {
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
private fun RenderBlock(block: RenderBlockDto) {
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

private fun parseDocument(content: String): List<RenderBlockDto>? {
    return try {
        val doc = DocumentHandle.fromString(content)
        doc.getSnapshot().blocks
    } catch (e: Exception) {
        Log.e(TAG, "Error parsing document", e)
        null
    }
}
