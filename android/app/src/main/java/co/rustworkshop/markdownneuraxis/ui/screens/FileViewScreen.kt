package co.rustworkshop.markdownneuraxis.ui.screens

import android.content.Intent
import android.net.Uri
import android.util.Log
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.ClickableText
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.readFileContent
import co.rustworkshop.markdownneuraxis.io.resolveDocumentFile
import co.rustworkshop.markdownneuraxis.model.FileTree
import kotlinx.coroutines.launch
import uniffi.markdown_neuraxis_ffi.DocumentHandle
import uniffi.markdown_neuraxis_ffi.RenderBlockDto
import uniffi.markdown_neuraxis_ffi.TextSegmentDto
import uniffi.markdown_neuraxis_ffi.resolveWikilink

private const val TAG = "MarkdownNeuraxis"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FileViewScreen(
    file: DocumentFile,
    fileTree: FileTree,
    notesUri: Uri,
    onBack: () -> Unit,
    onNavigateToFile: (DocumentFile) -> Unit
) {
    BackHandler(onBack = onBack)

    val context = LocalContext.current
    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()

    val content = remember(file) {
        readFileContent(context, file)
    }
    val snapshot = remember(content) {
        content?.let { parseDocument(it) }
    }

    val onWikiLinkClick: (String) -> Unit = { linkTarget ->
        val resolvedPath = resolveWikilink(linkTarget, fileTree.getAllFilePaths())
        if (resolvedPath != null) {
            val docFile = resolveDocumentFile(context, notesUri, resolvedPath)
            if (docFile != null) {
                onNavigateToFile(docFile)
            } else {
                coroutineScope.launch {
                    snackbarHostState.showSnackbar("\"$linkTarget\" not found")
                }
            }
        } else {
            coroutineScope.launch {
                snackbarHostState.showSnackbar("\"$linkTarget\" not found")
            }
        }
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
        },
        snackbarHost = { SnackbarHost(snackbarHostState) }
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
                        RenderBlock(block, onWikiLinkClick)
                    }
                }
            }
        }
    }
}

@Composable
private fun RenderSegments(
    segments: List<TextSegmentDto>,
    content: String,
    style: TextStyle = LocalTextStyle.current,
    modifier: Modifier = Modifier,
    onWikiLinkClick: (String) -> Unit
) {
    val context = LocalContext.current

    if (segments.isEmpty()) {
        Text(text = content, style = style, modifier = modifier)
        return
    }

    val linkColor = MaterialTheme.colorScheme.primary
    val annotatedText = buildAnnotatedString {
        for (segment in segments) {
            when (segment.kind) {
                "text" -> append(segment.content)
                "wiki_link" -> {
                    pushStringAnnotation(tag = "wiki_link", annotation = segment.content)
                    withStyle(SpanStyle(color = linkColor)) {
                        append("[[${segment.content}]]")
                    }
                    pop()
                }
                "url" -> {
                    pushStringAnnotation(tag = "url", annotation = segment.content)
                    withStyle(SpanStyle(color = linkColor, textDecoration = TextDecoration.Underline)) {
                        append(segment.content)
                    }
                    pop()
                }
                else -> append(segment.content)
            }
        }
    }

    ClickableText(
        text = annotatedText,
        style = style,
        modifier = modifier,
        onClick = { offset ->
            annotatedText.getStringAnnotations(tag = "url", start = offset, end = offset)
                .firstOrNull()?.let { annotation ->
                    try {
                        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(annotation.item))
                        context.startActivity(intent)
                    } catch (e: Exception) {
                        Log.e(TAG, "Error opening URL: ${annotation.item}", e)
                    }
                    return@ClickableText
                }
            annotatedText.getStringAnnotations(tag = "wiki_link", start = offset, end = offset)
                .firstOrNull()?.let { annotation ->
                    onWikiLinkClick(annotation.item)
                }
        }
    )
}

@Composable
private fun RenderBlock(block: RenderBlockDto, onWikiLinkClick: (String) -> Unit) {
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
            RenderSegments(
                segments = block.segments,
                content = block.content,
                style = style.copy(fontWeight = FontWeight.Bold),
                modifier = Modifier.padding(vertical = 8.dp),
                onWikiLinkClick = onWikiLinkClick
            )
        }
        "list_item" -> {
            Row(modifier = Modifier.padding(start = indent, top = 4.dp, bottom = 4.dp)) {
                Text(
                    text = block.listMarker ?: "-",
                    modifier = Modifier.width(24.dp)
                )
                RenderSegments(
                    segments = block.segments,
                    content = block.content,
                    onWikiLinkClick = onWikiLinkClick
                )
            }
        }
        "paragraph" -> {
            RenderSegments(
                segments = block.segments,
                content = block.content,
                modifier = Modifier.padding(vertical = 4.dp),
                onWikiLinkClick = onWikiLinkClick
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
                RenderSegments(
                    segments = block.segments,
                    content = block.content,
                    style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Light),
                    modifier = Modifier.padding(start = 16.dp, top = 8.dp, bottom = 8.dp, end = 8.dp),
                    onWikiLinkClick = onWikiLinkClick
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
