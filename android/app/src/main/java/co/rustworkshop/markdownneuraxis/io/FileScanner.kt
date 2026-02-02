package co.rustworkshop.markdownneuraxis.io

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import android.util.Log
import androidx.documentfile.provider.DocumentFile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield

private const val TAG = "MarkdownNeuraxis"
private const val FILE_SCAN_BATCH_SIZE = 20
private const val FILE_CACHE_NAME = "file_cache.txt"

/**
 * Data class to hold file path segments for tree building.
 * DocumentFile is no longer resolved during scanning for performance;
 * it is resolved on demand when the user clicks a file.
 */
data class FileWithPath(
    val file: DocumentFile?,
    val pathSegments: List<String>
)

/**
 * Progressively scan for markdown files, calling onBatch for each batch of files found.
 * Uses DocumentsContract + ContentResolver.query() directly instead of DocumentFile.listFiles()
 * for dramatically better performance (single cursor query per folder vs N+1 queries).
 */
suspend fun scanMarkdownFilesProgressively(
    context: Context,
    folderUri: Uri,
    onBatch: (List<FileWithPath>) -> Unit
) {
    withContext(Dispatchers.IO) {
        val rootDocId = DocumentsContract.getTreeDocumentId(folderUri)
        val mainThreadCallback: suspend (List<FileWithPath>) -> Unit = { files ->
            withContext(Dispatchers.Main) {
                onBatch(files)
            }
        }
        scanFolderRecursively(context, folderUri, rootDocId, emptyList(), mainThreadCallback)
    }
}

/**
 * Recursively scan a folder for markdown files using DocumentsContract for fast
 * cursor-based directory listing. Sibling subfolders are scanned in parallel
 * using coroutines, so IPC latency is overlapped across branches of the tree.
 */
private suspend fun scanFolderRecursively(
    context: Context,
    treeUri: Uri,
    parentDocId: String,
    pathPrefix: List<String>,
    onBatch: suspend (List<FileWithPath>) -> Unit
) {
    val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
    val projection = arrayOf(
        DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        DocumentsContract.Document.COLUMN_MIME_TYPE
    )

    val cursor = context.contentResolver.query(childrenUri, projection, null, null, null)
    if (cursor == null) {
        Log.w(TAG, "ContentResolver query returned null for: $childrenUri")
        return
    }

    val subfolders = mutableListOf<Pair<String, String>>() // docId to name
    val fileBatch = mutableListOf<FileWithPath>()

    cursor.use {
        val idIndex = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
        val nameIndex = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        val mimeIndex = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE)

        while (it.moveToNext()) {
            val docId = it.getString(idIndex) ?: continue
            val name = it.getString(nameIndex) ?: continue
            val mimeType = it.getString(mimeIndex) ?: ""

            if (mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
                subfolders.add(docId to name)
            } else if (name.endsWith(".md")) {
                fileBatch.add(FileWithPath(null, pathPrefix + name))
                if (fileBatch.size >= FILE_SCAN_BATCH_SIZE) {
                    onBatch(fileBatch.toList())
                    fileBatch.clear()
                    yield()
                }
            }
        }
    }

    // Emit remaining files from this folder
    if (fileBatch.isNotEmpty()) {
        onBatch(fileBatch.toList())
    }

    // Scan subfolders in parallel
    if (subfolders.isNotEmpty()) {
        coroutineScope {
            subfolders.map { (docId, name) ->
                async {
                    scanFolderRecursively(context, treeUri, docId, pathPrefix + name, onBatch)
                }
            }.awaitAll()
        }
    }
}

/**
 * Resolve a relative path to a DocumentFile by navigating from the root folder.
 * Uses DocumentsContract queries for fast lookup instead of DocumentFile.findFile()
 * which does a full directory listing per path segment.
 */
fun resolveDocumentFile(context: Context, notesUri: Uri, relativePath: String): DocumentFile? {
    val segments = relativePath.split("/")
    var currentDocId = DocumentsContract.getTreeDocumentId(notesUri)

    for (segment in segments) {
        val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(notesUri, currentDocId)
        val projection = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME
        )
        var foundId: String? = null
        context.contentResolver.query(childrenUri, projection, null, null, null)?.use { cursor ->
            val idIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val nameIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
            while (cursor.moveToNext()) {
                if (cursor.getString(nameIndex) == segment) {
                    foundId = cursor.getString(idIndex)
                    break
                }
            }
        }
        currentDocId = foundId ?: return null
    }

    val docUri = DocumentsContract.buildDocumentUriUsingTree(notesUri, currentDocId)
    return DocumentFile.fromSingleUri(context, docUri)
}

fun readFileContent(context: Context, file: DocumentFile): String? {
    return try {
        context.contentResolver.openInputStream(file.uri)?.use { stream ->
            stream.bufferedReader().readText()
        }
    } catch (e: Exception) {
        Log.e(TAG, "Error reading file: ${file.uri}", e)
        null
    }
}

fun loadFileCache(context: Context): List<String> {
    val cacheFile = java.io.File(context.filesDir, FILE_CACHE_NAME)
    return if (cacheFile.exists()) {
        try {
            cacheFile.readLines().filter { it.isNotBlank() }
        } catch (e: Exception) {
            Log.w(TAG, "Error loading file cache", e)
            emptyList()
        }
    } else {
        emptyList()
    }
}

fun clearFileCache(context: Context) {
    val cacheFile = java.io.File(context.filesDir, FILE_CACHE_NAME)
    cacheFile.delete()
}

fun saveFileCache(context: Context, paths: List<String>) {
    val cacheFile = java.io.File(context.filesDir, FILE_CACHE_NAME)
    try {
        cacheFile.writeText(paths.joinToString("\n"))
        Log.d(TAG, "Saved ${paths.size} paths to cache")
    } catch (e: Exception) {
        Log.e(TAG, "Error saving file cache", e)
    }
}
