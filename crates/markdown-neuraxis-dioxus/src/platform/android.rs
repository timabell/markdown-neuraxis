//! Android-specific platform functionality
//!
//! Handles folder picking and SAF-based file I/O using JNI.
//! See ADR-0010 for details on the folder picker implementation.
//! See ADR-0011 for the SAF IO abstraction approach.

use jni::JNIEnv;
use jni::objects::{JObject, JValue};
use markdown_neuraxis_engine::io::{IoError, IoProvider};
use relative_path::{RelativePath, RelativePathBuf};

/// Helper to run JNI operations with proper error handling
fn with_jni<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&mut JNIEnv, JObject) -> Result<T, jni::errors::Error>,
{
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    match f(&mut env, context) {
        Ok(result) => Some(result),
        Err(e) => {
            log::error!("JNI error: {e}");
            None
        }
    }
}

// ============================================================================
// Folder Picker (see ADR-0010, ADR-0011)
// ============================================================================

const FOLDER_PICKER_CLASS: &str = "co/rustworkshop/markdown_neuraxis/FolderPickerActivity";

/// Launch the native folder picker activity.
///
/// Returns `true` if the picker was launched successfully.
/// Use `is_folder_picker_complete()` to check when the user has made a selection,
/// then `get_folder_picker_result()` to retrieve the selected path.
pub fn launch_folder_picker() -> bool {
    with_jni(|env, context| {
        // Reset picker state before launching
        reset_folder_picker_internal(env)?;

        // Create intent to launch FolderPickerActivity
        let picker_class = env.find_class(FOLDER_PICKER_CLASS)?;
        let intent_class = env.find_class("android/content/Intent")?;
        let intent = env.new_object(
            intent_class,
            "(Landroid/content/Context;Ljava/lang/Class;)V",
            &[
                JValue::Object(&context),
                JValue::Object(&picker_class.into()),
            ],
        )?;

        // Add FLAG_ACTIVITY_NEW_TASK since we're starting from non-Activity context
        let flag_new_task: i32 = 0x10000000;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(flag_new_task)],
        )?;

        // Start the activity
        env.call_method(
            &context,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;

        log::info!("Launched folder picker activity");
        Ok(())
    })
    .is_some()
}

/// Check if the folder picker has completed (user selected or cancelled).
pub fn is_folder_picker_complete() -> bool {
    with_jni(|env, _context| {
        let picker_class = env.find_class(FOLDER_PICKER_CLASS)?;
        let completed = env.get_static_field(picker_class, "completed", "Z")?;
        completed.z()
    })
    .unwrap_or(false)
}

/// Get the result from the folder picker.
///
/// Returns `Some(path)` if a folder was selected, `None` if cancelled or not yet complete.
pub fn get_folder_picker_result() -> Option<String> {
    with_jni(|env, _context| {
        let picker_class = env.find_class(FOLDER_PICKER_CLASS)?;
        let result = env.get_static_field(picker_class, "result", "Ljava/lang/String;")?;
        let result_obj = result.l()?;

        if result_obj.is_null() {
            return Ok(None);
        }

        let result_jstring: jni::objects::JString = result_obj.into();
        let result_str = env.get_string(&result_jstring)?;
        Ok(Some(result_str.to_str().unwrap_or("").to_string()))
    })
    .flatten()
}

/// Reset the folder picker state for a new selection.
pub fn reset_folder_picker() {
    let _ = with_jni(|env, _context| reset_folder_picker_internal(env));
}

fn reset_folder_picker_internal(env: &mut JNIEnv) -> Result<(), jni::errors::Error> {
    let picker_class = env.find_class(FOLDER_PICKER_CLASS)?;

    // Get field IDs
    let completed_field = env.get_static_field_id(&picker_class, "completed", "Z")?;
    let result_field = env.get_static_field_id(&picker_class, "result", "Ljava/lang/String;")?;

    // Set completed = false
    env.set_static_field(&picker_class, completed_field, JValue::Bool(0))?;

    // Set result = null
    env.set_static_field(
        &picker_class,
        result_field,
        JValue::Object(&JObject::null()),
    )?;

    Ok(())
}

// ============================================================================
// SafProvider - Storage Access Framework based IoProvider (see ADR-0011)
// ============================================================================

/// Android SAF-based IoProvider implementation.
///
/// Uses ContentResolver via JNI to access files through SAF content URIs.
/// This allows file access without requiring MANAGE_EXTERNAL_STORAGE permission.
pub struct SafProvider {
    /// The content URI of the root folder (from folder picker)
    tree_uri: String,
    /// Cached display name for the root
    display_name: String,
}

impl SafProvider {
    /// Create a new SafProvider for the given content URI.
    ///
    /// The URI should be a tree URI from ACTION_OPEN_DOCUMENT_TREE with
    /// persistable permissions taken.
    pub fn new(tree_uri: String) -> Self {
        // Extract display name from URI
        let display_name = Self::extract_display_name(&tree_uri);
        Self {
            tree_uri,
            display_name,
        }
    }

    /// Get the tree URI
    pub fn tree_uri(&self) -> &str {
        &self.tree_uri
    }

    /// Extract a human-readable display name from the URI
    fn extract_display_name(tree_uri: &str) -> String {
        // Tree URI format: content://com.android.externalstorage.documents/tree/primary%3ADocuments%2Fnotes
        // We want to extract "notes" or whatever the folder name is
        if let Some(decoded) = urlencoding::decode(tree_uri).ok() {
            if let Some(last_part) = decoded.rsplit('/').next() {
                // Handle "primary:Documents/notes" -> "notes"
                if let Some(folder) = last_part.rsplit('/').next() {
                    if let Some(name) = folder.rsplit(':').next() {
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                    return folder.to_string();
                }
            }
        }
        "Notes".to_string()
    }

    /// Find or create a DocumentFile for the given relative path
    fn find_document_file<'a>(
        env: &mut JNIEnv<'a>,
        context: &JObject<'a>,
        tree_uri: &str,
        relative_path: &RelativePath,
    ) -> Result<JObject<'a>, jni::errors::Error> {
        // Parse the tree URI
        let uri = parse_uri(env, tree_uri)?;

        // Get the root DocumentFile
        let doc_file_class = env.find_class("androidx/documentfile/provider/DocumentFile")?;
        let root_doc = env.call_static_method(
            doc_file_class,
            "fromTreeUri",
            "(Landroid/content/Context;Landroid/net/Uri;)Landroidx/documentfile/provider/DocumentFile;",
            &[JValue::Object(context), JValue::Object(&uri)],
        )?.l()?;

        if root_doc.is_null() {
            return Err(jni::errors::Error::NullPtr("fromTreeUri returned null"));
        }

        // Navigate to the target file through path components
        let path_str = relative_path.as_str();
        if path_str.is_empty() {
            return Ok(root_doc);
        }

        let mut current = root_doc;
        let components: Vec<&str> = path_str.split('/').collect();

        for component in components {
            let name = env.new_string(component)?;
            let child = env
                .call_method(
                    &current,
                    "findFile",
                    "(Ljava/lang/String;)Landroidx/documentfile/provider/DocumentFile;",
                    &[JValue::Object(&name.into())],
                )?
                .l()?;

            if child.is_null() {
                return Err(jni::errors::Error::NullPtr("File not found"));
            }
            current = child;
        }

        Ok(current)
    }

    /// Create a file and any missing parent directories
    fn create_file<'a>(
        env: &mut JNIEnv<'a>,
        context: &JObject<'a>,
        tree_uri: &str,
        relative_path: &RelativePath,
    ) -> Result<JObject<'a>, jni::errors::Error> {
        let uri = parse_uri(env, tree_uri)?;

        let doc_file_class = env.find_class("androidx/documentfile/provider/DocumentFile")?;
        let root_doc = env.call_static_method(
            doc_file_class,
            "fromTreeUri",
            "(Landroid/content/Context;Landroid/net/Uri;)Landroidx/documentfile/provider/DocumentFile;",
            &[JValue::Object(context), JValue::Object(&uri)],
        )?.l()?;

        if root_doc.is_null() {
            return Err(jni::errors::Error::NullPtr("fromTreeUri returned null"));
        }

        let path_str = relative_path.as_str();
        let components: Vec<&str> = path_str.split('/').collect();

        if components.is_empty() {
            return Err(jni::errors::Error::NullPtr("Empty path"));
        }

        let mut current = root_doc;

        // Create directories for all but the last component
        for &component in &components[..components.len() - 1] {
            let name = env.new_string(component)?;

            // Try to find existing directory
            let child = env
                .call_method(
                    &current,
                    "findFile",
                    "(Ljava/lang/String;)Landroidx/documentfile/provider/DocumentFile;",
                    &[JValue::Object(&name.into())],
                )?
                .l()?;

            if child.is_null() {
                // Create directory
                let name = env.new_string(component)?;
                let new_dir = env
                    .call_method(
                        &current,
                        "createDirectory",
                        "(Ljava/lang/String;)Landroidx/documentfile/provider/DocumentFile;",
                        &[JValue::Object(&name.into())],
                    )?
                    .l()?;

                if new_dir.is_null() {
                    return Err(jni::errors::Error::NullPtr("Failed to create directory"));
                }
                current = new_dir;
            } else {
                current = child;
            }
        }

        // Create the file
        let filename = components.last().unwrap();
        let name = env.new_string(*filename)?;

        // Check if file already exists
        let existing = env
            .call_method(
                &current,
                "findFile",
                "(Ljava/lang/String;)Landroidx/documentfile/provider/DocumentFile;",
                &[JValue::Object(&name.into())],
            )?
            .l()?;

        if !existing.is_null() {
            return Ok(existing);
        }

        // Create new file with text/markdown mime type
        let name = env.new_string(*filename)?;
        let mime_type = env.new_string("text/markdown")?;
        let new_file = env.call_method(
            &current,
            "createFile",
            "(Ljava/lang/String;Ljava/lang/String;)Landroidx/documentfile/provider/DocumentFile;",
            &[
                JValue::Object(&mime_type.into()),
                JValue::Object(&name.into()),
            ],
        )?.l()?;

        if new_file.is_null() {
            return Err(jni::errors::Error::NullPtr("Failed to create file"));
        }

        Ok(new_file)
    }

    /// Read content from an InputStream into a String.
    /// Ensures the stream is closed even on error.
    fn read_input_stream(env: &mut JNIEnv, input_stream: &JObject) -> Result<String, IoError> {
        let mut bytes = Vec::new();
        let buffer_size = 8192i32;
        let buffer = env
            .new_byte_array(buffer_size)
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

        // Use a closure to capture the read logic, ensuring we can close stream after
        let result = (|| -> Result<Vec<u8>, IoError> {
            loop {
                let bytes_read = env
                    .call_method(input_stream, "read", "([B)I", &[JValue::Object(&buffer)])
                    .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                    .i()
                    .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

                if bytes_read < 0 {
                    break;
                }

                let mut chunk = vec![0i8; bytes_read as usize];
                env.get_byte_array_region(&buffer, 0, &mut chunk)
                    .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

                bytes.extend(chunk.iter().map(|&b| b as u8));
            }
            Ok(bytes)
        })();

        // Always close the stream, regardless of success or failure
        let _ = env.call_method(input_stream, "close", "()V", &[]);

        let bytes = result?;
        String::from_utf8(bytes)
            .map_err(|e| IoError::Io(std::io::Error::other(format!("UTF-8 error: {e}"))))
    }

    /// Write content to an OutputStream.
    /// Ensures the stream is closed even on error.
    fn write_output_stream(
        env: &mut JNIEnv,
        output_stream: &JObject,
        content: &str,
    ) -> Result<(), IoError> {
        // Use a closure to capture the write logic, ensuring we can close stream after
        let result = (|| -> Result<(), IoError> {
            let bytes = content.as_bytes();
            let java_bytes: Vec<i8> = bytes.iter().map(|&b| b as i8).collect();
            let byte_array = env
                .new_byte_array(java_bytes.len() as i32)
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            env.set_byte_array_region(&byte_array, 0, &java_bytes)
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            env.call_method(
                output_stream,
                "write",
                "([B)V",
                &[JValue::Object(&byte_array)],
            )
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            // Flush before closing
            let _ = env.call_method(output_stream, "flush", "()V", &[]);

            Ok(())
        })();

        // Always close the stream, regardless of success or failure
        let _ = env.call_method(output_stream, "close", "()V", &[]);

        result
    }
}

/// Parse a URI string into a Java Uri object
fn parse_uri<'a>(env: &mut JNIEnv<'a>, uri_str: &str) -> Result<JObject<'a>, jni::errors::Error> {
    let uri_jstring = env.new_string(uri_str)?;
    let uri_class = env.find_class("android/net/Uri")?;
    let uri = env
        .call_static_method(
            uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&uri_jstring.into())],
        )?
        .l()?;
    Ok(uri)
}

impl IoProvider for SafProvider {
    fn read_file(&self, relative_path: &RelativePath) -> Result<String, IoError> {
        let tree_uri = self.tree_uri.clone();
        let path = relative_path.to_owned();

        with_jni(|env, context| {
            // Find the document file
            let doc_file = Self::find_document_file(env, &context, &tree_uri, &path)
                .map_err(|e| IoError::NotFound(format!("{}: {e}", path.as_str())))?;

            // Get the file's URI
            let file_uri = env
                .call_method(&doc_file, "getUri", "()Landroid/net/Uri;", &[])
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            // Get ContentResolver
            let content_resolver = env
                .call_method(
                    &context,
                    "getContentResolver",
                    "()Landroid/content/ContentResolver;",
                    &[],
                )
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            // Open input stream
            let input_stream = env
                .call_method(
                    &content_resolver,
                    "openInputStream",
                    "(Landroid/net/Uri;)Ljava/io/InputStream;",
                    &[JValue::Object(&file_uri)],
                )
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            if input_stream.is_null() {
                return Err(IoError::NotFound(path.as_str().to_string()));
            }

            Self::read_input_stream(env, &input_stream)
        })
        .ok_or_else(|| IoError::NotFound(relative_path.to_string()))?
    }

    fn write_file(&self, relative_path: &RelativePath, content: &str) -> Result<(), IoError> {
        let tree_uri = self.tree_uri.clone();
        let path = relative_path.to_owned();
        let content = content.to_string();

        with_jni(|env, context| {
            // Create file (and parent directories)
            let doc_file = Self::create_file(env, &context, &tree_uri, &path).map_err(|e| {
                IoError::Io(std::io::Error::other(format!("Failed to create file: {e}")))
            })?;

            // Get the file's URI
            let file_uri = env
                .call_method(&doc_file, "getUri", "()Landroid/net/Uri;", &[])
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            // Get ContentResolver
            let content_resolver = env
                .call_method(
                    &context,
                    "getContentResolver",
                    "()Landroid/content/ContentResolver;",
                    &[],
                )
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            // Open output stream (with "wt" mode to truncate)
            let mode = env
                .new_string("wt")
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;
            let output_stream = env
                .call_method(
                    &content_resolver,
                    "openOutputStream",
                    "(Landroid/net/Uri;Ljava/lang/String;)Ljava/io/OutputStream;",
                    &[JValue::Object(&file_uri), JValue::Object(&mode.into())],
                )
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            if output_stream.is_null() {
                return Err(IoError::Io(std::io::Error::other(
                    "Failed to open output stream",
                )));
            }

            Self::write_output_stream(env, &output_stream, &content)
        })
        .ok_or_else(|| IoError::Io(std::io::Error::other("JNI context unavailable")))?
    }

    fn list_markdown_files(&self) -> Result<Vec<RelativePathBuf>, IoError> {
        let tree_uri = self.tree_uri.clone();

        with_jni(|env, context| {
            let uri = parse_uri(env, &tree_uri)
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            let doc_file_class = env
                .find_class("androidx/documentfile/provider/DocumentFile")
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            let root_doc = env
                .call_static_method(
                    doc_file_class,
                    "fromTreeUri",
                    "(Landroid/content/Context;Landroid/net/Uri;)Landroidx/documentfile/provider/DocumentFile;",
                    &[JValue::Object(&context), JValue::Object(&uri)],
                )
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            if root_doc.is_null() {
                return Err(IoError::InvalidNotesDir(
                    "Failed to open document tree".to_string(),
                ));
            }

            let mut files = Vec::new();
            list_files_recursive(env, &root_doc, &RelativePathBuf::new(), &mut files)?;

            files.sort();
            Ok(files)
        })
        .ok_or_else(|| IoError::Io(std::io::Error::other("JNI context unavailable")))?
    }

    fn exists(&self, relative_path: &RelativePath) -> bool {
        let tree_uri = self.tree_uri.clone();
        let path = relative_path.to_owned();

        with_jni(|env, context| Self::find_document_file(env, &context, &tree_uri, &path).is_ok())
            .unwrap_or(false)
    }

    fn validate(&self) -> Result<(), IoError> {
        let tree_uri = self.tree_uri.clone();

        with_jni(|env, context| {
            let uri = parse_uri(env, &tree_uri)
                .map_err(|e| IoError::InvalidNotesDir(format!("Invalid URI: {e}")))?;

            let doc_file_class = env
                .find_class("androidx/documentfile/provider/DocumentFile")
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            let root_doc = env
                .call_static_method(
                    doc_file_class,
                    "fromTreeUri",
                    "(Landroid/content/Context;Landroid/net/Uri;)Landroidx/documentfile/provider/DocumentFile;",
                    &[JValue::Object(&context), JValue::Object(&uri)],
                )
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .l()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            if root_doc.is_null() {
                return Err(IoError::InvalidNotesDir(
                    "Cannot access document tree".to_string(),
                ));
            }

            // Check if it exists and is a directory
            let exists = env
                .call_method(&root_doc, "exists", "()Z", &[])
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .z()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            let is_directory = env
                .call_method(&root_doc, "isDirectory", "()Z", &[])
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
                .z()
                .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

            if !exists || !is_directory {
                return Err(IoError::InvalidNotesDir(
                    "Path does not exist or is not a directory".to_string(),
                ));
            }

            Ok(())
        })
        .ok_or_else(|| IoError::Io(std::io::Error::other("JNI context unavailable")))?
    }

    fn root_display_name(&self) -> String {
        self.display_name.clone()
    }
}

/// Recursively list markdown files in a DocumentFile tree
fn list_files_recursive(
    env: &mut JNIEnv,
    doc_file: &JObject,
    current_path: &RelativePathBuf,
    files: &mut Vec<RelativePathBuf>,
) -> Result<(), IoError> {
    let children = env
        .call_method(
            doc_file,
            "listFiles",
            "()[Landroidx/documentfile/provider/DocumentFile;",
            &[],
        )
        .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
        .l()
        .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

    if children.is_null() {
        return Ok(());
    }

    let children_array = unsafe { jni::objects::JObjectArray::from_raw(children.as_raw()) };
    let length = env
        .get_array_length(&children_array)
        .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

    for i in 0..length {
        let child = env
            .get_object_array_element(&children_array, i)
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

        // Get name
        let name_obj = env
            .call_method(&child, "getName", "()Ljava/lang/String;", &[])
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
            .l()
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

        if name_obj.is_null() {
            continue;
        }

        let name_jstring: jni::objects::JString = name_obj.into();
        let name = env
            .get_string(&name_jstring)
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;
        let name_str = name.to_str().unwrap_or("");

        let child_path = current_path.join(name_str);

        let is_directory = env
            .call_method(&child, "isDirectory", "()Z", &[])
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?
            .z()
            .map_err(|e| IoError::Io(std::io::Error::other(format!("JNI error: {e}"))))?;

        if is_directory {
            list_files_recursive(env, &child, &child_path, files)?;
        } else if name_str.ends_with(".md") {
            files.push(child_path);
        }
    }

    Ok(())
}
