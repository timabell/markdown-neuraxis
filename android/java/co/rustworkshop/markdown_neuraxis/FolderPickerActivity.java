package co.rustworkshop.markdown_neuraxis;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;

/**
 * Helper Activity for native folder selection using Storage Access Framework.
 *
 * Launches the system folder picker, takes persistable URI permissions,
 * and returns the content URI for use with ContentResolver.
 *
 * See ADR-0010 for why this helper Activity is needed.
 * See ADR-0011 for the SAF-based IO abstraction approach.
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

        // Launch the system folder picker with persistable permission flags
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        intent.addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        intent.addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        startActivityForResult(intent, REQUEST_CODE_OPEN_DOCUMENT_TREE);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        if (requestCode == REQUEST_CODE_OPEN_DOCUMENT_TREE) {
            if (resultCode == RESULT_OK && data != null) {
                Uri uri = data.getData();
                if (uri != null) {
                    // Take persistable permission for access across app restarts
                    int takeFlags = Intent.FLAG_GRANT_READ_URI_PERMISSION
                            | Intent.FLAG_GRANT_WRITE_URI_PERMISSION;
                    getContentResolver().takePersistableUriPermission(uri, takeFlags);

                    // Return the content URI directly (not converted to path)
                    // The Rust SafProvider will use this with ContentResolver
                    result = uri.toString();
                }
            }
            completed = true;
            finish();
        }
    }
}
