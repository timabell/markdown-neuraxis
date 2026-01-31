package co.rustworkshop.markdownneuraxis.model

import androidx.documentfile.provider.DocumentFile

/**
 * Represents a node in the file tree - either a folder or a file.
 * Stores relative path for cache persistence; DocumentFile resolved on demand.
 */
sealed class FileTreeNode {
    abstract val name: String
    abstract val depth: Int
    abstract val relativePath: String

    data class Folder(
        override val name: String,
        override val depth: Int,
        override val relativePath: String,
        val children: MutableList<FileTreeNode> = mutableListOf(),
        var isExpanded: Boolean = false
    ) : FileTreeNode()

    data class File(
        override val name: String,
        override val depth: Int,
        override val relativePath: String,
        var documentFile: DocumentFile? = null
    ) : FileTreeNode()
}

/**
 * Manages the file tree structure for progressive loading
 */
class FileTree {
    private val root: MutableList<FileTreeNode> = mutableListOf()
    private val folderMap: MutableMap<String, FileTreeNode.Folder> = mutableMapOf()
    private val fileMap: MutableMap<String, FileTreeNode.File> = mutableMapOf()

    /**
     * Add a file from path segments only (for cache loading).
     * DocumentFile will be null until resolved on demand.
     */
    fun addFilePath(pathSegments: List<String>) {
        addFileInternal(pathSegments, null)
    }

    /**
     * Add a file with its DocumentFile (from scanning).
     */
    fun addFile(file: DocumentFile, pathSegments: List<String>) {
        addFileInternal(pathSegments, file)
    }

    private fun addFileInternal(pathSegments: List<String>, documentFile: DocumentFile?) {
        if (pathSegments.isEmpty()) return

        val relativePath = pathSegments.joinToString("/")

        // Skip if already exists (cache may have it, scan will update)
        if (fileMap.containsKey(relativePath)) {
            if (documentFile != null) {
                fileMap[relativePath]?.documentFile = documentFile
            }
            return
        }

        var currentChildren = root
        var currentDepth = 0

        // Navigate/create folders for all but the last segment
        for (i in 0 until pathSegments.size - 1) {
            val folderName = pathSegments[i]
            val pathKey = pathSegments.subList(0, i + 1).joinToString("/")

            val existingFolder = folderMap[pathKey]
            if (existingFolder != null) {
                currentChildren = existingFolder.children
            } else {
                val newFolder = FileTreeNode.Folder(
                    name = folderName,
                    depth = currentDepth,
                    relativePath = pathKey,
                    isExpanded = false
                )
                folderMap[pathKey] = newFolder
                currentChildren.add(newFolder)
                sortChildren(currentChildren)
                currentChildren = newFolder.children
            }
            currentDepth++
        }

        val fileName = pathSegments.last()
        val fileNode = FileTreeNode.File(
            name = fileName,
            depth = currentDepth,
            relativePath = relativePath,
            documentFile = documentFile
        )
        fileMap[relativePath] = fileNode
        currentChildren.add(fileNode)
        sortChildren(currentChildren)
    }

    private fun sortChildren(children: MutableList<FileTreeNode>) {
        children.sortBy { node ->
            when (node) {
                is FileTreeNode.Folder -> "0_${node.name.lowercase()}"
                is FileTreeNode.File -> "1_${node.name.lowercase()}"
            }
        }
    }

    fun getRootNodes(): List<FileTreeNode> = root.toList()

    fun toggleFolder(folder: FileTreeNode.Folder) {
        folder.isExpanded = !folder.isExpanded
    }

    fun getAllFilePaths(): List<String> = fileMap.keys.toList().sorted()

    fun getFileCount(): Int = fileMap.size

    /**
     * Remove files that are no longer present (not in scannedPaths).
     * Returns count of removed files.
     */
    fun removeStaleFiles(scannedPaths: Set<String>): Int {
        val stalePaths = fileMap.keys.filter { it !in scannedPaths }
        for (path in stalePaths) {
            removeFile(path)
        }
        return stalePaths.size
    }

    private fun removeFile(relativePath: String) {
        fileMap.remove(relativePath) ?: return

        val segments = relativePath.split("/")
        if (segments.size == 1) {
            root.removeIf { it is FileTreeNode.File && it.relativePath == relativePath }
        } else {
            val parentPath = segments.dropLast(1).joinToString("/")
            val parentFolder = folderMap[parentPath]
            parentFolder?.children?.removeIf { it is FileTreeNode.File && it.relativePath == relativePath }
            cleanupEmptyFolders()
        }
    }

    private fun cleanupEmptyFolders() {
        val foldersToRemove = mutableListOf<String>()
        for ((path, folder) in folderMap) {
            if (folder.children.isEmpty()) {
                foldersToRemove.add(path)
            }
        }

        for (path in foldersToRemove.sortedByDescending { it.count { c -> c == '/' } }) {
            folderMap.remove(path) ?: continue
            val segments = path.split("/")
            if (segments.size == 1) {
                root.removeIf { it is FileTreeNode.Folder && it.relativePath == path }
            } else {
                val parentPath = segments.dropLast(1).joinToString("/")
                val parentFolder = folderMap[parentPath]
                parentFolder?.children?.removeIf { it is FileTreeNode.Folder && it.relativePath == path }
            }
        }

        if (foldersToRemove.isNotEmpty()) {
            cleanupEmptyFolders()
        }
    }

    fun getFlattenedList(): List<FileTreeNode> {
        val result = mutableListOf<FileTreeNode>()
        fun flatten(nodes: List<FileTreeNode>) {
            for (node in nodes) {
                result.add(node)
                if (node is FileTreeNode.Folder && node.isExpanded) {
                    flatten(node.children)
                }
            }
        }
        flatten(root)
        return result
    }
}

/**
 * State for progressive file discovery
 */
data class FileDiscoveryState(
    val tree: FileTree = FileTree(),
    val fileCount: Int = 0,
    val isScanning: Boolean = false,
    val error: String? = null
)
