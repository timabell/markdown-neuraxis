package co.rustworkshop.markdownneuraxis.ui.screens

import android.content.Intent
import android.net.Uri
import android.util.Log
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.ClickableText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
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
import androidx.compose.ui.graphics.Color
import androidx.compose.material3.LocalContentColor
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.readFileContent
import co.rustworkshop.markdownneuraxis.io.resolveDocumentFile
import co.rustworkshop.markdownneuraxis.model.FileTree
import uniffi.markdown_neuraxis_ffi.Block
import uniffi.markdown_neuraxis_ffi.DocumentHandle
import uniffi.markdown_neuraxis_ffi.TextSegment
import uniffi.markdown_neuraxis_ffi.resolveWikilink

private const val TAG = "MarkdownNeuraxis"

/** Extract plain text from segments recursively */
private fun segmentsToText(segments: List<TextSegment>): String {
    return segments.joinToString("") { segment ->
        when (segment.kind) {
            "text", "code", "strikethrough", "wiki_link" -> segment.content
            "emphasis", "strong" -> segmentsToText(segment.children)
            "link", "image" -> segment.content.substringBefore("|")
            "hard_break" -> "\n"
            "soft_break" -> " "
            else -> ""
        }
    }
}

@Composable
fun FileViewScreen(
    file: DocumentFile,
    fileTree: FileTree,
    notesUri: Uri,
    onNavigateToFile: (DocumentFile) -> Unit,
    onNavigateToFolder: (String) -> Unit,
    onMissingFile: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val content = remember(file) {
        readFileContent(context, file)
    }
    val blocks = remember(content) {
        content?.let { parseDocument(it) }
    }

    val onWikiLinkClick: (String) -> Unit = { linkTarget ->
        // First check if target matches a folder
        val folder = fileTree.findFolderByName(linkTarget)
        if (folder != null) {
            onNavigateToFolder(folder.relativePath)
        } else {
            // Not a folder, resolve as file
            val resolvedPath = resolveWikilink(linkTarget, fileTree.getAllFilePaths())
            if (resolvedPath != null) {
                // Expand parent folders so file is visible
                fileTree.expandToFile(resolvedPath)
                val docFile = resolveDocumentFile(context, notesUri, resolvedPath)
                if (docFile != null) {
                    onNavigateToFile(docFile)
                } else {
                    onMissingFile(linkTarget)
                }
            } else {
                onMissingFile(linkTarget)
            }
        }
    }

    when {
        content == null -> {
            Box(
                modifier = modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Text("Error reading file")
            }
        }
        blocks == null -> {
            Box(
                modifier = modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Text("Error parsing document")
            }
        }
        else -> {
            Column(
                modifier = modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp)
                    .verticalScroll(rememberScrollState())
            ) {
                RenderBlockTree(blocks, depth = 0, onWikiLinkClick = onWikiLinkClick)
            }
        }
    }
}

/**
 * Recursively render a list of blocks and their children.
 * List blocks handle their own children (for proper index-based markers).
 */
@Composable
private fun RenderBlockTree(
    blocks: List<Block>,
    depth: Int,
    onWikiLinkClick: (String) -> Unit
) {
    for (block in blocks) {
        RenderBlock(block, depth, onWikiLinkClick)
        // List blocks handle their own children internally
        if (block.kind != "list" && block.children.isNotEmpty()) {
            RenderBlockTree(block.children, depth + 1, onWikiLinkClick)
        }
    }
}

@Composable
private fun RenderSegments(
    segments: List<TextSegment>,
    style: TextStyle = LocalTextStyle.current,
    modifier: Modifier = Modifier,
    onWikiLinkClick: (String) -> Unit
) {
    val context = LocalContext.current
    val contentColor = LocalContentColor.current

    // Ensure style has the correct text color for dark/light mode
    val themedStyle = if (style.color == Color.Unspecified) {
        style.copy(color = contentColor)
    } else {
        style
    }

    if (segments.isEmpty()) {
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
                        append(segment.content)
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
                "soft_break" -> append(" ")
                "hard_break" -> append("\n")
                else -> append(segment.content)
            }
        }
    }

    ClickableText(
        text = annotatedText,
        style = themedStyle,
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
private fun RenderBlock(block: Block, depth: Int, onWikiLinkClick: (String) -> Unit) {
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
                style = style.copy(fontWeight = FontWeight.Bold),
                modifier = Modifier.padding(vertical = 8.dp),
                onWikiLinkClick = onWikiLinkClick
            )
        }
        "list" -> {
            // List container renders its children with index-generated markers
            // Nesting indent comes from marker width - nested content is after marker
            val ordered = block.listOrdered == true
            Column {
                block.children.forEachIndexed { index, item ->
                    val marker = if (ordered) "${index + 1}." else "•"
                    val markerWidth = if (ordered) 24.dp else 16.dp
                    Row(modifier = Modifier.padding(top = 4.dp, bottom = 4.dp)) {
                        Text(
                            text = marker,
                            modifier = Modifier.width(markerWidth)
                        )
                        Column {
                            RenderSegments(
                                segments = item.segments,
                                onWikiLinkClick = onWikiLinkClick
                            )
                            // Render nested content (e.g., nested lists)
                            if (item.children.isNotEmpty()) {
                                RenderBlockTree(item.children, depth + 1, onWikiLinkClick)
                            }
                        }
                    }
                }
            }
        }
        "list_item" -> {
            // Standalone list_item should never happen - list container handles its items
            error("list_item rendered outside of list container - invalid block structure")
        }
        "paragraph" -> {
            RenderSegments(
                segments = block.segments,
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
                    text = segmentsToText(block.segments),
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
                text = segmentsToText(block.segments),
                modifier = Modifier.padding(vertical = 4.dp)
            )
        }
    }
}

private fun parseDocument(content: String): List<Block>? {
    return try {
        val doc = DocumentHandle.fromString(content)
        doc.getSnapshot().blocks
    } catch (e: Exception) {
        Log.e(TAG, "Error parsing document", e)
        null
    }
}
