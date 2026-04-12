package co.rustworkshop.markdownneuraxis.ui.screens

import android.content.Intent
import android.net.Uri
import android.util.Log
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.ClickableText
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.graphics.Color
import androidx.compose.material3.LocalContentColor
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.readFileContent
import co.rustworkshop.markdownneuraxis.io.resolveDocumentFile
import co.rustworkshop.markdownneuraxis.io.writeFileContent
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

    // Mutable content state that can be updated after edits
    var content by remember(file) {
        mutableStateOf(readFileContent(context, file))
    }
    var blocks by remember(file) {
        mutableStateOf(content?.let { parseDocument(it) })
    }

    // Edit state
    var editingBlockId by remember { mutableStateOf<String?>(null) }
    var editText by remember { mutableStateOf(TextFieldValue("")) }
    var editSourceRange by remember { mutableStateOf<Pair<Int, Int>?>(null) }

    // Save edit and refresh document
    val saveEdit: () -> Unit = saveEdit@{
        val range = editSourceRange
        val currentContent = content
        if (range != null && currentContent != null && editingBlockId != null) {
            // Convert content to UTF-8 bytes for correct byte-offset indexing
            val utf8Bytes = currentContent.toByteArray(Charsets.UTF_8)

            // Bounds check - invalid range is a bug, crash to catch during development
            require(range.first >= 0 && range.second <= utf8Bytes.size && range.first <= range.second) {
                "Invalid byte range: ${range.first}..${range.second} for content of ${utf8Bytes.size} bytes"
            }

            // Extract original source text to check for changes
            val originalText = String(utf8Bytes, range.first, range.second - range.first, Charsets.UTF_8)
            // Check if original had trailing newline (to restore it)
            val hadTrailingNewline = originalText.endsWith("\n")
            val originalWithoutNewline = originalText.removeSuffix("\n")

            // Only save if content actually changed
            if (editText.text != originalWithoutNewline) {
                // Restore trailing newline if original had one
                val textToSave = if (hadTrailingNewline) editText.text + "\n" else editText.text
                // Replace the byte range with new text
                val before = String(utf8Bytes, 0, range.first, Charsets.UTF_8)
                val after = String(utf8Bytes, range.second, utf8Bytes.size - range.second, Charsets.UTF_8)
                val newContent = before + textToSave + after

                // Write to file
                if (writeFileContent(context, file, newContent)) {
                    // Update state with new content
                    content = newContent
                    blocks = parseDocument(newContent)
                } else {
                    Log.e(TAG, "Failed to save edit")
                }
            }
        }
        // Clear edit state
        editingBlockId = null
        editText = TextFieldValue("")
        editSourceRange = null
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
            val currentBlocks = blocks!! // Safe: we're in else branch where blocks != null
            Column(
                modifier = modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp)
                    .verticalScroll(rememberScrollState())
            ) {
                RenderBlockTree(
                    blocks = currentBlocks,
                    depth = 0,
                    onWikiLinkClick = onWikiLinkClick,
                    editingBlockId = editingBlockId,
                    editText = editText,
                    onStartEdit = { blockId, start, end ->
                        // Save current edit before starting new one
                        if (editingBlockId != null) {
                            saveEdit()
                        }
                        // Extract raw source text using byte offsets
                        val currentContent = content ?: return@RenderBlockTree
                        val utf8Bytes = currentContent.toByteArray(Charsets.UTF_8)
                        // Bounds check - invalid range is a bug
                        require(start >= 0 && end <= utf8Bytes.size && start <= end) {
                            "Invalid byte range: $start..$end for content of ${utf8Bytes.size} bytes"
                        }
                        val sourceText = String(utf8Bytes, start, end - start, Charsets.UTF_8)
                        // Strip trailing newline for editing (will restore on save)
                        val editableText = sourceText.removeSuffix("\n")
                        editingBlockId = blockId
                        editText = TextFieldValue(editableText, TextRange(editableText.length))
                        editSourceRange = Pair(start, end)
                    },
                    onEditTextChange = { editText = it },
                    onFinishEdit = saveEdit
                )
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
    onWikiLinkClick: (String) -> Unit,
    editingBlockId: String?,
    editText: TextFieldValue,
    onStartEdit: (blockId: String, start: Int, end: Int) -> Unit,
    onEditTextChange: (TextFieldValue) -> Unit,
    onFinishEdit: () -> Unit
) {
    for (block in blocks) {
        RenderBlock(
            block = block,
            depth = depth,
            onWikiLinkClick = onWikiLinkClick,
            editingBlockId = editingBlockId,
            editText = editText,
            onStartEdit = onStartEdit,
            onEditTextChange = onEditTextChange,
            onFinishEdit = onFinishEdit
        )
        // These blocks handle their own children internally
        val handlesOwnChildren = block.kind in listOf("list", "table", "table_header_row", "table_row", "block_quote")
        if (!handlesOwnChildren && block.children.isNotEmpty()) {
            RenderBlockTree(
                blocks = block.children,
                depth = depth + 1,
                onWikiLinkClick = onWikiLinkClick,
                editingBlockId = editingBlockId,
                editText = editText,
                onStartEdit = onStartEdit,
                onEditTextChange = onEditTextChange,
                onFinishEdit = onFinishEdit
            )
        }
    }
}

@Composable
private fun RenderSegments(
    segments: List<TextSegment>,
    style: TextStyle = LocalTextStyle.current,
    modifier: Modifier = Modifier,
    onWikiLinkClick: (String) -> Unit,
    onTextClick: (() -> Unit)? = null
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
                    return@ClickableText
                }
            // No link clicked - call text click handler if provided
            onTextClick?.invoke()
        }
    )
}

@Composable
private fun RenderBlock(
    block: Block,
    depth: Int,
    onWikiLinkClick: (String) -> Unit,
    editingBlockId: String?,
    editText: TextFieldValue,
    onStartEdit: (blockId: String, start: Int, end: Int) -> Unit,
    onEditTextChange: (TextFieldValue) -> Unit,
    onFinishEdit: () -> Unit
) {
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
            val isEditing = editingBlockId == block.id
            if (isEditing) {
                val focusRequester = remember { FocusRequester() }
                var hasFocused by remember { mutableStateOf(false) }
                BasicTextField(
                    value = editText,
                    onValueChange = onEditTextChange,
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                        .padding(8.dp)
                        .focusRequester(focusRequester)
                        .onFocusChanged { focusState ->
                            if (focusState.isFocused) {
                                hasFocused = true
                            } else if (hasFocused) {
                                onFinishEdit()
                            }
                        },
                    textStyle = style.copy(
                        fontWeight = FontWeight.Bold,
                        color = LocalContentColor.current
                    ),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { onFinishEdit() })
                )
                LaunchedEffect(Unit) {
                    focusRequester.requestFocus()
                }
            } else {
                RenderSegments(
                    segments = block.segments,
                    style = style.copy(fontWeight = FontWeight.Bold),
                    modifier = Modifier.padding(vertical = 8.dp),
                    onWikiLinkClick = onWikiLinkClick,
                    onTextClick = {
                        onStartEdit(
                            block.id,
                            block.sourceStart.toInt(),
                            block.sourceEnd.toInt()
                        )
                    }
                )
            }
        }
        "list" -> {
            // List container renders its children with index-generated markers
            // Nesting indent comes from marker width - nested content is after marker
            val ordered = block.listOrdered == true
            Column {
                block.children.forEachIndexed { index, item ->
                    val marker = if (ordered) "${index + 1}." else "•"
                    val markerWidth = if (ordered) 24.dp else 16.dp
                    val isEditing = editingBlockId == item.id
                    Row(modifier = Modifier.padding(top = 4.dp, bottom = 4.dp)) {
                        Text(
                            text = marker,
                            modifier = Modifier.width(markerWidth)
                        )
                        Column {
                            if (isEditing) {
                                val focusRequester = remember { FocusRequester() }
                                var hasFocused by remember { mutableStateOf(false) }
                                BasicTextField(
                                    value = editText,
                                    onValueChange = onEditTextChange,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .border(1.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                                        .padding(8.dp)
                                        .focusRequester(focusRequester)
                                        .onFocusChanged { focusState ->
                                            if (focusState.isFocused) {
                                                hasFocused = true
                                            } else if (hasFocused) {
                                                onFinishEdit()
                                            }
                                        },
                                    textStyle = LocalTextStyle.current.copy(
                                        color = LocalContentColor.current
                                    ),
                                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                                    keyboardActions = KeyboardActions(onDone = { onFinishEdit() })
                                )
                                LaunchedEffect(Unit) {
                                    focusRequester.requestFocus()
                                }
                            } else {
                                RenderSegments(
                                    segments = item.segments,
                                    onWikiLinkClick = onWikiLinkClick,
                                    onTextClick = {
                                        onStartEdit(
                                            item.id,
                                            item.sourceStart.toInt(),
                                            item.sourceEnd.toInt()
                                        )
                                    }
                                )
                            }
                            // Render nested content (e.g., nested lists)
                            if (item.children.isNotEmpty()) {
                                RenderBlockTree(
                                    blocks = item.children,
                                    depth = depth + 1,
                                    onWikiLinkClick = onWikiLinkClick,
                                    editingBlockId = editingBlockId,
                                    editText = editText,
                                    onStartEdit = onStartEdit,
                                    onEditTextChange = onEditTextChange,
                                    onFinishEdit = onFinishEdit
                                )
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
            val isEditing = editingBlockId == block.id
            if (isEditing) {
                // Edit mode: show text field with visible border
                val focusRequester = remember { FocusRequester() }
                var hasFocused by remember { mutableStateOf(false) }
                BasicTextField(
                    value = editText,
                    onValueChange = onEditTextChange,
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                        .padding(8.dp)
                        .focusRequester(focusRequester)
                        .onFocusChanged { focusState ->
                            if (focusState.isFocused) {
                                hasFocused = true
                            } else if (hasFocused) {
                                // Only save when focus is lost after having focus
                                onFinishEdit()
                            }
                        },
                    textStyle = LocalTextStyle.current.copy(
                        color = LocalContentColor.current
                    ),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { onFinishEdit() })
                )
                LaunchedEffect(Unit) {
                    focusRequester.requestFocus()
                }
            } else {
                // View mode: tappable text (tap non-link area to edit)
                RenderSegments(
                    segments = block.segments,
                    modifier = Modifier.padding(vertical = 4.dp),
                    onWikiLinkClick = onWikiLinkClick,
                    onTextClick = {
                        onStartEdit(
                            block.id,
                            block.sourceStart.toInt(),
                            block.sourceEnd.toInt()
                        )
                    }
                )
            }
        }
        "code_fence" -> {
            val isEditing = editingBlockId == block.id
            if (isEditing) {
                val focusRequester = remember { FocusRequester() }
                var hasFocused by remember { mutableStateOf(false) }
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = MaterialTheme.shapes.small,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .border(2.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                ) {
                    BasicTextField(
                        value = editText,
                        onValueChange = onEditTextChange,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(8.dp)
                            .focusRequester(focusRequester)
                            .onFocusChanged { focusState ->
                                if (focusState.isFocused) {
                                    hasFocused = true
                                } else if (hasFocused) {
                                    onFinishEdit()
                                }
                            },
                        textStyle = MaterialTheme.typography.bodySmall.copy(
                            fontFamily = FontFamily.Monospace,
                            color = LocalContentColor.current
                        ),
                        // Code blocks may be multiline, use default IME
                        keyboardOptions = KeyboardOptions.Default
                    )
                }
                LaunchedEffect(Unit) {
                    focusRequester.requestFocus()
                }
            } else {
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = MaterialTheme.shapes.small,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .clickable {
                            onStartEdit(
                                block.id,
                                block.sourceStart.toInt(),
                                block.sourceEnd.toInt()
                            )
                        }
                ) {
                    Text(
                        text = segmentsToText(block.segments),
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.padding(8.dp)
                    )
                }
            }
        }
        "block_quote" -> {
            val isEditing = editingBlockId == block.id
            if (isEditing) {
                // Edit the entire blockquote as raw markdown
                val focusRequester = remember { FocusRequester() }
                var hasFocused by remember { mutableStateOf(false) }
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .border(2.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                ) {
                    BasicTextField(
                        value = editText,
                        onValueChange = onEditTextChange,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(start = 16.dp, top = 8.dp, bottom = 8.dp, end = 8.dp)
                            .focusRequester(focusRequester)
                            .onFocusChanged { focusState ->
                                if (focusState.isFocused) {
                                    hasFocused = true
                                } else if (hasFocused) {
                                    onFinishEdit()
                                }
                            },
                        textStyle = MaterialTheme.typography.bodyMedium.copy(
                            fontWeight = FontWeight.Light,
                            color = LocalContentColor.current
                        ),
                        // Blockquotes may be multiline
                        keyboardOptions = KeyboardOptions.Default
                    )
                }
                LaunchedEffect(Unit) {
                    focusRequester.requestFocus()
                }
            } else {
                // Render blockquote with children inside the Surface
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .clickable {
                            onStartEdit(
                                block.id,
                                block.sourceStart.toInt(),
                                block.sourceEnd.toInt()
                            )
                        }
                ) {
                    Column(
                        modifier = Modifier.padding(start = 16.dp, top = 8.dp, bottom = 8.dp, end = 8.dp)
                    ) {
                        // Render any direct segments
                        if (block.segments.isNotEmpty()) {
                            RenderSegments(
                                segments = block.segments,
                                style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Light),
                                onWikiLinkClick = onWikiLinkClick
                            )
                        }
                        // Render child blocks (paragraphs inside the quote)
                        for (child in block.children) {
                            RenderSegments(
                                segments = child.segments,
                                style = MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Light),
                                onWikiLinkClick = onWikiLinkClick
                            )
                        }
                    }
                }
            }
        }
        "thematic_break" -> {
            val isEditing = editingBlockId == block.id
            if (isEditing) {
                val focusRequester = remember { FocusRequester() }
                var hasFocused by remember { mutableStateOf(false) }
                BasicTextField(
                    value = editText,
                    onValueChange = onEditTextChange,
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                        .padding(8.dp)
                        .focusRequester(focusRequester)
                        .onFocusChanged { focusState ->
                            if (focusState.isFocused) {
                                hasFocused = true
                            } else if (hasFocused) {
                                onFinishEdit()
                            }
                        },
                    textStyle = LocalTextStyle.current.copy(
                        color = LocalContentColor.current
                    ),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { onFinishEdit() })
                )
                LaunchedEffect(Unit) {
                    focusRequester.requestFocus()
                }
            } else {
                HorizontalDivider(
                    modifier = Modifier
                        .padding(vertical = 8.dp)
                        .clickable {
                            onStartEdit(
                                block.id,
                                block.sourceStart.toInt(),
                                block.sourceEnd.toInt()
                            )
                        }
                )
            }
        }
        "table" -> {
            // Table container - calculate column count for consistent widths
            val columnCount = block.children.firstOrNull()?.children?.size ?: 1
            Column(
                modifier = Modifier
                    .padding(vertical = 8.dp)
                    .border(1.dp, MaterialTheme.colorScheme.outline)
            ) {
                block.children.forEachIndexed { index, row ->
                    RenderTableRow(
                        block = row,
                        columnCount = columnCount,
                        onWikiLinkClick = onWikiLinkClick,
                        editingBlockId = editingBlockId,
                        editText = editText,
                        onStartEdit = onStartEdit,
                        onEditTextChange = onEditTextChange,
                        onFinishEdit = onFinishEdit
                    )
                    // Add divider between rows (not after last)
                    if (index < block.children.size - 1) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                    }
                }
            }
        }
        "table_header_row", "table_row" -> {
            // Rows are rendered via RenderTableRow, this handles standalone case
            RenderTableRow(
                block = block,
                columnCount = block.children.size,
                onWikiLinkClick = onWikiLinkClick,
                editingBlockId = editingBlockId,
                editText = editText,
                onStartEdit = onStartEdit,
                onEditTextChange = onEditTextChange,
                onFinishEdit = onFinishEdit
            )
        }
        "table_cell" -> {
            // Standalone cell (shouldn't happen - rows handle cells)
            RenderSegments(
                segments = block.segments,
                onWikiLinkClick = onWikiLinkClick
            )
        }
        else -> {
            val isEditing = editingBlockId == block.id
            if (isEditing) {
                val focusRequester = remember { FocusRequester() }
                var hasFocused by remember { mutableStateOf(false) }
                BasicTextField(
                    value = editText,
                    onValueChange = onEditTextChange,
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.small)
                        .padding(8.dp)
                        .focusRequester(focusRequester)
                        .onFocusChanged { focusState ->
                            if (focusState.isFocused) {
                                hasFocused = true
                            } else if (hasFocused) {
                                onFinishEdit()
                            }
                        },
                    textStyle = LocalTextStyle.current.copy(
                        color = LocalContentColor.current
                    ),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { onFinishEdit() })
                )
                LaunchedEffect(Unit) {
                    focusRequester.requestFocus()
                }
            } else {
                Text(
                    text = segmentsToText(block.segments),
                    modifier = Modifier
                        .padding(vertical = 4.dp)
                        .clickable {
                            onStartEdit(
                                block.id,
                                block.sourceStart.toInt(),
                                block.sourceEnd.toInt()
                            )
                        }
                )
            }
        }
    }
}

@Composable
private fun RenderTableRow(
    block: Block,
    columnCount: Int,
    onWikiLinkClick: (String) -> Unit,
    editingBlockId: String?,
    editText: TextFieldValue,
    onStartEdit: (blockId: String, start: Int, end: Int) -> Unit,
    onEditTextChange: (TextFieldValue) -> Unit,
    onFinishEdit: () -> Unit
) {
    val isHeader = block.kind == "table_header_row"
    val cells = block.children

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(IntrinsicSize.Min)
            .then(
                if (isHeader) Modifier.background(MaterialTheme.colorScheme.surfaceVariant)
                else Modifier
            )
    ) {
        // Render each cell, padding to columnCount if needed
        for (i in 0 until columnCount) {
            if (i > 0) {
                // Vertical divider between cells
                VerticalDivider(
                    modifier = Modifier.fillMaxHeight(),
                    color = MaterialTheme.colorScheme.outline
                )
            }
            Box(
                modifier = Modifier
                    .weight(1f)
                    .padding(horizontal = 8.dp, vertical = 6.dp)
            ) {
                if (i < cells.size) {
                    val cell = cells[i]
                    val isEditing = editingBlockId == cell.id
                    if (isEditing) {
                        val focusRequester = remember { FocusRequester() }
                        var hasFocused by remember { mutableStateOf(false) }
                        BasicTextField(
                            value = editText,
                            onValueChange = onEditTextChange,
                            modifier = Modifier
                                .fillMaxWidth()
                                .border(1.dp, MaterialTheme.colorScheme.primary, MaterialTheme.shapes.extraSmall)
                                .padding(4.dp)
                                .focusRequester(focusRequester)
                                .onFocusChanged { focusState ->
                                    if (focusState.isFocused) {
                                        hasFocused = true
                                    } else if (hasFocused) {
                                        onFinishEdit()
                                    }
                                },
                            textStyle = if (isHeader) {
                                MaterialTheme.typography.bodyMedium.copy(
                                    fontWeight = FontWeight.Bold,
                                    color = LocalContentColor.current
                                )
                            } else {
                                MaterialTheme.typography.bodyMedium.copy(
                                    color = LocalContentColor.current
                                )
                            },
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = { onFinishEdit() })
                        )
                        LaunchedEffect(Unit) {
                            focusRequester.requestFocus()
                        }
                    } else {
                        RenderSegments(
                            segments = cell.segments,
                            style = if (isHeader) {
                                MaterialTheme.typography.bodyMedium.copy(fontWeight = FontWeight.Bold)
                            } else {
                                MaterialTheme.typography.bodyMedium
                            },
                            onWikiLinkClick = onWikiLinkClick,
                            onTextClick = {
                                onStartEdit(
                                    cell.id,
                                    cell.sourceStart.toInt(),
                                    cell.sourceEnd.toInt()
                                )
                            }
                        )
                    }
                }
            }
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
