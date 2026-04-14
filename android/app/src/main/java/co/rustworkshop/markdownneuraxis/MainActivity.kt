package co.rustworkshop.markdownneuraxis

import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.*
import androidx.compose.material3.LocalContentColor
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.documentfile.provider.DocumentFile
import co.rustworkshop.markdownneuraxis.io.*
import co.rustworkshop.markdownneuraxis.model.FileDiscoveryState
import co.rustworkshop.markdownneuraxis.ui.components.AppBottomBar
import co.rustworkshop.markdownneuraxis.ui.components.AppDrawerContent
import co.rustworkshop.markdownneuraxis.ui.screens.FileListScreen
import co.rustworkshop.markdownneuraxis.ui.screens.FileViewScreen
import co.rustworkshop.markdownneuraxis.ui.screens.MissingFileScreen
import co.rustworkshop.markdownneuraxis.ui.screens.NewFileScreen
import co.rustworkshop.markdownneuraxis.ui.screens.SetupScreen
import co.rustworkshop.markdownneuraxis.ui.theme.MarkdownNeuraxisTheme
import kotlinx.coroutines.launch

/**
 * State for unsaved new files. These exist only in memory until saved.
 * @param relativePath intended path e.g., "folder/new.md"
 * @param content unsaved content
 */
data class NewFileState(
	val relativePath: String,
	val content: String = ""
)

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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun App() {
	val context = LocalContext.current
	var notesUri by remember { mutableStateOf(getValidNotesUri(context)) }
	val fileStack = remember { mutableStateListOf<DocumentFile>() }
	var missingFileName by remember { mutableStateOf<String?>(null) }
	var previousUri by remember { mutableStateOf<Uri?>(null) }

	var discoveryState by remember { mutableStateOf(FileDiscoveryState()) }
	var treeVersion by remember { mutableIntStateOf(0) }
	var hasScannedThisSession by remember { mutableStateOf(false) }

	// New file state (unsaved file in memory)
	var newFileState by remember { mutableStateOf<NewFileState?>(null) }

	// Title editing state for rename
	var isEditingTitle by remember { mutableStateOf(false) }
	var editingTitleText by remember { mutableStateOf(TextFieldValue("")) }

	val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
	val scope = rememberCoroutineScope()

	val isSetup = notesUri == null
	val hasMissing = missingFileName != null
	val hasNewFile = newFileState != null
	val hasFile = fileStack.isNotEmpty()

	// Editing state from FileViewScreen
	var isEditing by remember { mutableStateOf(false) }
	var saveEditCallback by remember { mutableStateOf<(() -> Unit)?>(null) }

	// Create new file in folder
	val onNewFile: (folderRelativePath: String) -> Unit = onNewFile@{ folderPath ->
		val uri = notesUri ?: return@onNewFile
		val folderDocId = getFolderDocId(context, uri, folderPath)
			?: DocumentsContract.getTreeDocumentId(uri)
		val filename = generateUniqueFilename(context, uri, folderDocId)
		val relativePath = if (folderPath.isEmpty()) filename else "$folderPath/$filename"
		newFileState = NewFileState(relativePath, "")
		// Start with title editing - select just filename so typing replaces it
		val displayPath = relativePath.removeSuffix(".md")
		val filenameStart = displayPath.lastIndexOf('/') + 1
		editingTitleText = TextFieldValue(displayPath, TextRange(filenameStart, displayPath.length))
		isEditingTitle = true
	}

	// Save new file to disk - returns true on success, false on clash
	val onSaveNewFile: (content: String) -> Boolean = saveNewFile@{ content ->
		val state = checkNotNull(newFileState) { "onSaveNewFile: newFileState is null" }
		val uri = checkNotNull(notesUri) { "onSaveNewFile: notesUri is null" }

		// Check if target already exists
		if (fileExists(context, uri, state.relativePath)) {
			Toast.makeText(context, "File already exists: ${state.relativePath}", Toast.LENGTH_SHORT).show()
			return@saveNewFile false
		}

		val segments = state.relativePath.split("/")
		val parentPath = if (segments.size > 1) segments.dropLast(1).joinToString("/") else ""
		val filename = segments.last()

		// Resolve or create parent folder
		val parentFolder = if (parentPath.isEmpty()) {
			DocumentFile.fromTreeUri(context, uri)
		} else {
			createFolderPath(context, uri, parentPath)
		}
		checkNotNull(parentFolder) { "Failed to resolve parent folder: $parentPath" }

		// Create the file
		val newFile = createNewFile(context, uri, parentFolder, filename)
		checkNotNull(newFile) { "Failed to create file: ${state.relativePath}" }

		// Write content
		check(writeFileContent(context, newFile, content)) {
			newFile.delete()
			"Failed to write content to: ${state.relativePath}"
		}

		// Add to file tree and navigate
		discoveryState.tree.addFile(newFile, segments)
		treeVersion++
		fileStack.add(newFile)
		newFileState = null
		saveFileCache(context, discoveryState.tree.getAllFilePaths())
		true
	}

	// Commit title edit for new file - updates intended path, checks for clash
	val commitNewFileTitleEdit: () -> Unit = {
		if (newFileState != null && notesUri != null) {
			val newPath = editingTitleText.text.trim().let {
				if (it.endsWith(".md")) it else "$it.md"
			}
			// Always update the path with what user typed
			newFileState = newFileState!!.copy(relativePath = newPath)
			if (fileExists(context, notesUri!!, newPath)) {
				Toast.makeText(context, "File already exists: $newPath", Toast.LENGTH_SHORT).show()
				// Stay in editing mode so user can fix
			} else {
				isEditingTitle = false
			}
		} else {
			isEditingTitle = false
		}
	}

	// Rename/move file - returns true on success, false on clash
	val onRenameFile: (oldPath: String, newPath: String) -> Boolean = renameFile@{ oldPath, newPath ->
		val uri = checkNotNull(notesUri) { "onRenameFile: notesUri is null" }

		// Check if target already exists - user needs to pick another name
		if (fileExists(context, uri, newPath)) {
			Toast.makeText(context, "File already exists: $newPath", Toast.LENGTH_SHORT).show()
			return@renameFile false
		}

		// Move the file
		val newFile = moveFile(context, uri, oldPath, newPath)
		checkNotNull(newFile) { "Failed to move file from $oldPath to $newPath" }

		// Update tree: remove old, add new with DocumentFile
		discoveryState.tree.removeFile(oldPath)
		discoveryState.tree.addFile(newFile, newPath.split("/"))
		treeVersion++

		// Update file stack
		if (fileStack.isNotEmpty()) {
			fileStack.removeAt(fileStack.lastIndex)
			fileStack.add(newFile)
		}

		saveFileCache(context, discoveryState.tree.getAllFilePaths())
		true
	}

	// Commit title edit for existing file - renames from editingTitleText
	val commitExistingFileTitleEdit: () -> Unit = {
		var success = true
		if (fileStack.isNotEmpty()) {
			val currentFile = fileStack.last()
			val currentPath = discoveryState.tree.findRelativePath(currentFile)
			if (currentPath != null) {
				val newPath = editingTitleText.text.trim().let {
					if (it.endsWith(".md")) it else "$it.md"
				}
				if (newPath != currentPath) {
					success = onRenameFile(currentPath, newPath)
				}
			}
		}
		if (success) {
			isEditingTitle = false
		}
	}

	BackHandler(enabled = drawerState.isOpen || hasFile || hasNewFile || hasMissing) {
		when {
			drawerState.isOpen -> scope.launch { drawerState.close() }
			hasMissing -> missingFileName = null
			hasNewFile -> {
				// Auto-save new file if it has content, stay on screen if clash
				val saved = if (newFileState!!.content.isNotBlank()) {
					onSaveNewFile(newFileState!!.content)
				} else {
					true // No content, just discard
				}
				if (saved) {
					newFileState = null
					isEditingTitle = false
				}
			}
			hasFile -> fileStack.removeAt(fileStack.lastIndex)
		}
	}

	Box(modifier = Modifier.fillMaxSize().imePadding()) {
		ModalNavigationDrawer(
			drawerState = drawerState,
			gesturesEnabled = !isSetup,
			drawerContent = {
				AppDrawerContent(
					onChangeFolder = {
						previousUri = notesUri
						notesUri = null
					},
					onCloseDrawer = { scope.launch { drawerState.close() } }
				)
			}
		) {
			Scaffold(
				topBar = {
					when {
						isSetup -> TopAppBar(title = { Text("Markdown Neuraxis") })
						hasMissing -> TopAppBar(
							title = { Text(missingFileName!!) }
						)
						hasNewFile -> {
							val focusRequester = remember { FocusRequester() }
							var hadFocus by remember { mutableStateOf(false) }
							TopAppBar(
								title = {
									if (isEditingTitle) {
										BasicTextField(
											value = editingTitleText,
											onValueChange = { editingTitleText = it },
											singleLine = true,
											textStyle = MaterialTheme.typography.titleLarge.copy(
												color = LocalContentColor.current
											),
											keyboardOptions = KeyboardOptions(
												// Uri keyboard type adds `/` for easy folder path editing
												keyboardType = KeyboardType.Uri,
												imeAction = ImeAction.Done
											),
											keyboardActions = KeyboardActions(
												onDone = {
													commitNewFileTitleEdit()
													hadFocus = false
												}
											),
											modifier = Modifier
												.focusRequester(focusRequester)
												.onFocusChanged { focusState ->
													if (focusState.isFocused) {
														hadFocus = true
													} else if (hadFocus && isEditingTitle) {
														commitNewFileTitleEdit()
														hadFocus = false
													}
												}
										)
										LaunchedEffect(Unit) {
											focusRequester.requestFocus()
										}
									} else {
										Text(
											text = newFileState!!.relativePath.removeSuffix(".md"),
											modifier = Modifier.clickable {
												val displayPath = newFileState!!.relativePath.removeSuffix(".md")
												editingTitleText = TextFieldValue(
													displayPath,
													TextRange(displayPath.length)
												)
												isEditingTitle = true
											}
										)
									}
								}
							)
						}
						hasFile -> {
							val currentFile = fileStack.last()
							val currentPath = discoveryState.tree.findRelativePath(currentFile)
							val focusRequester = remember { FocusRequester() }
							var hadFocus by remember { mutableStateOf(false) }
							TopAppBar(
								title = {
									if (isEditingTitle && currentPath != null) {
										BasicTextField(
											value = editingTitleText,
											onValueChange = { editingTitleText = it },
											singleLine = true,
											textStyle = MaterialTheme.typography.titleLarge.copy(
												color = LocalContentColor.current
											),
											keyboardOptions = KeyboardOptions(
												// Uri keyboard type adds `/` for easy folder path editing
												keyboardType = KeyboardType.Uri,
												imeAction = ImeAction.Done
											),
											keyboardActions = KeyboardActions(
												onDone = {
													commitExistingFileTitleEdit()
													hadFocus = false
												}
											),
											modifier = Modifier
												.focusRequester(focusRequester)
												.onFocusChanged { focusState ->
													if (focusState.isFocused) {
														hadFocus = true
													} else if (hadFocus && isEditingTitle) {
														commitExistingFileTitleEdit()
														hadFocus = false
													}
												}
										)
										LaunchedEffect(Unit) {
											focusRequester.requestFocus()
										}
									} else {
										Text(
											text = currentPath?.removeSuffix(".md")
												?: currentFile.name?.removeSuffix(".md")
												?: "File",
											modifier = Modifier.clickable {
												if (currentPath != null) {
													// Show full path for editing
													val displayPath = currentPath.removeSuffix(".md")
													editingTitleText = TextFieldValue(
														displayPath,
														TextRange(displayPath.length)
													)
													isEditingTitle = true
												}
											}
										)
									}
								}
							)
						}
						else -> TopAppBar(
							title = { Text("Notes") },
							actions = {
								IconButton(onClick = { onNewFile("") }) {
									Icon(Icons.Default.Add, contentDescription = "New file")
								}
							}
						)
					}
				},
				bottomBar = {
					if (!isSetup) {
						AppBottomBar(
							onMenuClick = { scope.launch { drawerState.open() } },
							onHomeClick = {
								// Auto-save new file if it has content, stay on screen if clash
								val saved = if (newFileState != null && newFileState!!.content.isNotBlank()) {
									onSaveNewFile(newFileState!!.content)
								} else {
									true
								}
								if (saved) {
									fileStack.clear()
									missingFileName = null
									newFileState = null
									isEditingTitle = false
								}
							},
							isEditing = isEditing ||
								hasNewFile ||
								(isEditingTitle && hasFile),
							onDoneClick = when {
								hasNewFile && isEditingTitle -> {
									// Commit title, transition to body if no clash
									{ commitNewFileTitleEdit() }
								}
								hasNewFile && !isEditingTitle -> {
									{
										onSaveNewFile(newFileState!!.content)
										Unit
									}
								}
								isEditingTitle && hasFile -> {
									{ commitExistingFileTitleEdit() }
								}
								else -> saveEditCallback
							}
						)
					}
				}
			) { padding ->
				when {
					isSetup -> {
						SetupScreen(
							onFolderSelected = { uri ->
								previousUri = null
								fileStack.clear()
								missingFileName = null
								discoveryState = FileDiscoveryState()
								hasScannedThisSession = false
								clearFileCache(context)
								saveNotesUri(context, uri)
								notesUri = uri
							},
							onCancel = previousUri?.let {
								{
									notesUri = it
									previousUri = null
								}
							},
							modifier = Modifier.padding(padding)
						)
					}
					hasMissing -> {
						MissingFileScreen(
							fileName = missingFileName!!,
							modifier = Modifier.padding(padding)
						)
					}
					hasNewFile -> {
						if (isEditingTitle) {
							// Show empty content while user picks filename
							Box(modifier = Modifier.padding(padding).fillMaxSize())
						} else {
							NewFileScreen(
								initialContent = newFileState!!.content,
								onContentChanged = { content ->
									newFileState = newFileState!!.copy(content = content)
								},
								modifier = Modifier.padding(padding),
								autoFocus = true
							)
						}
					}
					hasFile -> {
						FileViewScreen(
							file = fileStack.last(),
							fileTree = discoveryState.tree,
							notesUri = notesUri!!,
							onNavigateToFile = { file ->
								// Expand parent folders so file is visible, then navigate
								treeVersion++
								fileStack.add(file)
							},
							onNavigateToFolder = { folderPath ->
								// Find and expand the folder, then navigate back to file list
								discoveryState.tree.findFolderByName(folderPath)?.let { folder ->
									discoveryState.tree.expandToFolder(folder)
								}
								fileStack.clear()
								treeVersion++
							},
							onMissingFile = { name -> missingFileName = name },
							onEditingChanged = { editing, saveEdit ->
								isEditing = editing
								saveEditCallback = if (editing) saveEdit else null
							},
							modifier = Modifier.padding(padding)
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
							onFileSelected = { file -> fileStack.add(file) },
							onNewFile = onNewFile,
							modifier = Modifier.padding(padding)
						)
					}
				}
			}
		}
	}
}
