package co.rustworkshop.markdown_neuraxis;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;

/**
 * Helper Activity for native folder selection.
 *
 * Launches the system folder picker and stores the result in static fields
 * for retrieval via JNI from Rust code.
 *
 * See ADR-0010 for why this helper Activity is needed.
 */
public class FolderPickerActivity extends Activity {
    private static final int REQUEST_CODE_OPEN_DOCUMENT_TREE = 1;

    // Static fields accessed from Rust via JNI
    public static volatile String result = null;
    public static volatile boolean completed = false;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Reset state for new picker session
        result = null;
        completed = false;

        // Launch the system folder picker
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        intent.addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        startActivityForResult(intent, REQUEST_CODE_OPEN_DOCUMENT_TREE);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode == REQUEST_CODE_OPEN_DOCUMENT_TREE) {
            if (resultCode == RESULT_OK && data != null) {
                Uri uri = data.getData();
                if (uri != null) {
                    result = convertTreeUriToPath(uri);
                }
            }
            completed = true;
            finish();
        }
    }

    /**
     * Convert a document tree URI to a filesystem path.
     *
     * Example input:  content://com.android.externalstorage.documents/tree/primary:Documents/foo
     * Example output: /storage/emulated/0/Documents/foo
     *
     * This works because we have MANAGE_EXTERNAL_STORAGE permission.
     */
    private String convertTreeUriToPath(Uri uri) {
        String docId = getTreeDocumentId(uri);
        if (docId == null) {
            return null;
        }

        // Document ID format: "primary:path/to/folder" or "XXXX-XXXX:path/to/folder"
        String[] parts = docId.split(":", 2);
        if (parts.length < 2) {
            // Root of storage selected
            if ("primary".equals(parts[0])) {
                return "/storage/emulated/0";
            } else {
                // SD card or other storage
                return "/storage/" + parts[0];
            }
        }

        String storageId = parts[0];
        String relativePath = parts[1];

        if ("primary".equals(storageId)) {
            return "/storage/emulated/0/" + relativePath;
        } else {
            // SD card or other external storage
            return "/storage/" + storageId + "/" + relativePath;
        }
    }

    /**
     * Extract the document ID from a tree URI.
     */
    private String getTreeDocumentId(Uri uri) {
        String path = uri.getPath();
        if (path == null) {
            return null;
        }

        // Path format: /tree/primary:Documents/foo
        if (path.startsWith("/tree/")) {
            String encoded = path.substring(6); // Remove "/tree/"
            return Uri.decode(encoded);
        }

        return null;
    }
}
