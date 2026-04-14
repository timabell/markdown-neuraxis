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
suspend fun scanMarkdownFilesProgressively(context: Context, folderUri: Uri, onBatch: (List<FileWithPath>) -> Unit) {
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

fun writeFileContent(context: Context, file: DocumentFile, content: String): Boolean {
	return try {
		val stream = context.contentResolver.openOutputStream(file.uri, "wt")
		if (stream == null) {
			Log.e(TAG, "Failed to open output stream for: ${file.uri}")
			return false
		}
		stream.use { outputStream ->
			outputStream.bufferedWriter().use { writer ->
				writer.write(content)
			}
		}
		true
	} catch (e: Exception) {
		Log.e(TAG, "Error writing file: ${file.uri}", e)
		false
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

/**
 * Get the document ID for a folder by relative path.
 * Returns null if the folder doesn't exist.
 */
fun getFolderDocId(context: Context, notesUri: Uri, relativePath: String): String? {
	if (relativePath.isEmpty()) {
		return DocumentsContract.getTreeDocumentId(notesUri)
	}

	val segments = relativePath.split("/")
	var currentDocId = DocumentsContract.getTreeDocumentId(notesUri)

	for (segment in segments) {
		val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(notesUri, currentDocId)
		val projection = arrayOf(
			DocumentsContract.Document.COLUMN_DOCUMENT_ID,
			DocumentsContract.Document.COLUMN_DISPLAY_NAME,
			DocumentsContract.Document.COLUMN_MIME_TYPE
		)
		var foundId: String? = null
		context.contentResolver.query(childrenUri, projection, null, null, null)?.use { cursor ->
			val idIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
			val nameIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
			val mimeIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE)
			while (cursor.moveToNext()) {
				val name = cursor.getString(nameIndex)
				val mimeType = cursor.getString(mimeIndex) ?: ""
				if (name == segment && mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
					foundId = cursor.getString(idIndex)
					break
				}
			}
		}
		currentDocId = foundId ?: return null
	}

	return currentDocId
}

/**
 * Generate a unique filename in the given folder.
 * Returns "new.md", "new-1.md", "new-2.md", etc.
 */
fun generateUniqueFilename(context: Context, treeUri: Uri, folderDocId: String): String {
	val existingNames = mutableSetOf<String>()
	val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, folderDocId)
	val projection = arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME)

	context.contentResolver.query(childrenUri, projection, null, null, null)?.use { cursor ->
		val nameIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
		while (cursor.moveToNext()) {
			existingNames.add(cursor.getString(nameIndex).lowercase())
		}
	}

	if ("new.md" !in existingNames) {
		return "new.md"
	}

	var counter = 1
	while ("new-$counter.md" in existingNames) {
		counter++
	}
	return "new-$counter.md"
}

/**
 * Create a new markdown file in the given parent folder.
 * Returns the created DocumentFile or null on failure.
 * @param displayName The filename with .md extension (e.g., "new.md")
 * @param treeUri The root tree URI (needed for DocumentsContract operations)
 */
fun createNewFile(context: Context, treeUri: Uri, parentFolder: DocumentFile, displayName: String): DocumentFile? {
	return try {
		val parentDocId = DocumentsContract.getDocumentId(parentFolder.uri)
		// Use application/octet-stream to prevent SAF from adding extensions
		val newDocUri = DocumentsContract.createDocument(
			context.contentResolver,
			DocumentsContract.buildDocumentUriUsingTree(treeUri, parentDocId),
			"application/octet-stream",
			displayName
		)
		newDocUri?.let { DocumentFile.fromSingleUri(context, it) }
	} catch (e: Exception) {
		Log.e(TAG, "Error creating file: $displayName", e)
		null
	}
}

/**
 * Check if a file exists at the given relative path.
 */
fun fileExists(context: Context, notesUri: Uri, relativePath: String): Boolean {
	return resolveDocumentFile(context, notesUri, relativePath) != null
}

/**
 * Create nested folders as needed for the given relative path.
 * Returns the deepest folder's DocumentFile or null on failure.
 */
fun createFolderPath(context: Context, notesUri: Uri, relativePath: String): DocumentFile? {
	if (relativePath.isEmpty()) {
		return DocumentFile.fromTreeUri(context, notesUri)
	}

	val segments = relativePath.split("/")
	var currentDocId = DocumentsContract.getTreeDocumentId(notesUri)

	for (segment in segments) {
		val existingDocId = findChildFolderDocId(context, notesUri, currentDocId, segment)
		currentDocId = existingDocId ?: run {
			val parentDocUri = DocumentsContract.buildDocumentUriUsingTree(notesUri, currentDocId)
			val newFolderUri = DocumentsContract.createDocument(
				context.contentResolver,
				parentDocUri,
				DocumentsContract.Document.MIME_TYPE_DIR,
				segment
			) ?: return null
			DocumentsContract.getDocumentId(newFolderUri)
		}
	}

	val folderUri = DocumentsContract.buildDocumentUriUsingTree(notesUri, currentDocId)
	return DocumentFile.fromSingleUri(context, folderUri)
}

/**
 * Find a child folder's document ID by name within a parent folder.
 */
private fun findChildFolderDocId(context: Context, treeUri: Uri, parentDocId: String, folderName: String): String? {
	val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId)
	val projection = arrayOf(
		DocumentsContract.Document.COLUMN_DOCUMENT_ID,
		DocumentsContract.Document.COLUMN_DISPLAY_NAME,
		DocumentsContract.Document.COLUMN_MIME_TYPE
	)

	context.contentResolver.query(childrenUri, projection, null, null, null)?.use { cursor ->
		val idIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
		val nameIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
		val mimeIndex = cursor.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE)

		while (cursor.moveToNext()) {
			val name = cursor.getString(nameIndex)
			val mimeType = cursor.getString(mimeIndex)
			if (name == folderName && mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
				return cursor.getString(idIndex)
			}
		}
	}
	return null
}

/**
 * Move or rename a file. Handles both same-folder rename and cross-folder move.
 * For cross-folder moves: reads content, creates new file, writes content, deletes old.
 * Returns the new DocumentFile or null on failure.
 */
fun moveFile(context: Context, notesUri: Uri, oldPath: String, newPath: String): DocumentFile? {
	val oldFile = resolveDocumentFile(context, notesUri, oldPath) ?: return null

	val oldSegments = oldPath.split("/")
	val newSegments = newPath.split("/")
	val oldParentPath = if (oldSegments.size > 1) oldSegments.dropLast(1).joinToString("/") else ""
	val newParentPath = if (newSegments.size > 1) newSegments.dropLast(1).joinToString("/") else ""
	val newFileName = newSegments.last()

	// Same folder: just rename using DocumentsContract directly
	if (oldParentPath == newParentPath) {
		return try {
			val oldDocId = DocumentsContract.getDocumentId(oldFile.uri)
			val docUri = DocumentsContract.buildDocumentUriUsingTree(notesUri, oldDocId)
			val renamedUri = DocumentsContract.renameDocument(context.contentResolver, docUri, newFileName)
			if (renamedUri != null) {
				DocumentFile.fromSingleUri(context, renamedUri)
			} else {
				Log.e(TAG, "Failed to rename file from $oldPath to $newPath")
				null
			}
		} catch (e: Exception) {
			Log.e(TAG, "Error renaming file from $oldPath to $newPath", e)
			null
		}
	}

	// Cross-folder move: read content, create new, write, delete old
	val content = readFileContent(context, oldFile) ?: return null

	// Create parent folders if needed
	val newParentFolder = createFolderPath(context, notesUri, newParentPath) ?: return null

	// Create new file
	val newFile = createNewFile(context, notesUri, newParentFolder, newFileName) ?: return null

	// Write content
	if (!writeFileContent(context, newFile, content)) {
		newFile.delete()
		return null
	}

	// Delete old file
	check(oldFile.delete()) { "Failed to delete old file after move: $oldPath" }

	// Cleanup empty parent folders
	cleanupEmptyFolders(context, notesUri, oldParentPath)

	return newFile
}

/**
 * Remove empty parent folders up to the root.
 */
fun cleanupEmptyFolders(context: Context, notesUri: Uri, path: String) {
	if (path.isEmpty()) return

	val segments = path.split("/")

	// Work from deepest to shallowest
	for (i in segments.size downTo 1) {
		val folderPath = segments.take(i).joinToString("/")
		val folderDocId = getFolderDocId(context, notesUri, folderPath) ?: continue

		// Check if folder is empty
		val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(notesUri, folderDocId)
		val projection = arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
		var hasChildren = false

		context.contentResolver.query(childrenUri, projection, null, null, null)?.use { cursor ->
			hasChildren = cursor.moveToFirst()
		}

		if (!hasChildren) {
			val docUri = DocumentsContract.buildDocumentUriUsingTree(notesUri, folderDocId)
			try {
				DocumentsContract.deleteDocument(context.contentResolver, docUri)
				Log.d(TAG, "Deleted empty folder: $folderPath")
			} catch (e: Exception) {
				Log.w(TAG, "Failed to delete empty folder: $folderPath", e)
				break
			}
		} else {
			break
		}
	}
}
