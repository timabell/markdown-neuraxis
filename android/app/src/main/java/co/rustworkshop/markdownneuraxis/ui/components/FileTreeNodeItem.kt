package co.rustworkshop.markdownneuraxis.ui.components

import android.net.Uri
import android.util.Log
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.resolveDocumentFile
import co.rustworkshop.markdownneuraxis.model.FileTreeNode

private const val TAG = "MarkdownNeuraxis"

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
                        val docFile = node.documentFile
                            ?: resolveDocumentFile(context, notesUri, node.relativePath)
                        if (docFile != null) {
                            node.documentFile = docFile
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
