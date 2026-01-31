package co.rustworkshop.markdownneuraxis.io

import android.content.Context
import android.net.Uri
import android.util.Log

private const val TAG = "MarkdownNeuraxis"
private const val PREFS_NAME = "markdown_neuraxis_prefs"
private const val KEY_NOTES_URI = "notes_uri"

/**
 * Get the saved notes URI if it exists and the permission is still valid.
 * Returns null if no URI saved or permission was revoked.
 */
fun getValidNotesUri(context: Context): Uri? {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    val uriString = prefs.getString(KEY_NOTES_URI, null) ?: return null
    val uri = Uri.parse(uriString)

    val hasPermission = context.contentResolver.persistedUriPermissions.any {
        it.uri == uri && it.isReadPermission
    }

    if (!hasPermission) {
        Log.w(TAG, "Permission lost for saved URI: $uri")
        prefs.edit().remove(KEY_NOTES_URI).apply()
        return null
    }

    return uri
}

fun saveNotesUri(context: Context, uri: Uri) {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    prefs.edit().putString(KEY_NOTES_URI, uri.toString()).apply()
}
