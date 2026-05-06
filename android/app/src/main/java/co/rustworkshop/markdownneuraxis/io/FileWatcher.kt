package co.rustworkshop.markdownneuraxis.io

import android.content.Context
import android.database.ContentObserver
import android.os.Handler
import android.os.Looper
import androidx.documentfile.provider.DocumentFile
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/**
 * (mtime, size) pair used to detect external writes to an open file
 * (ADR-0014). The mtime is in epoch milliseconds as returned by
 * DocumentFile.lastModified(); size is bytes per DocumentFile.length().
 */
data class FileMeta(val mtime: Long, val size: Long)

/** One detected external change: the file content as it now reads on disk plus its meta. */
data class WatcherEvent(val content: String, val meta: FileMeta)

/**
 * Hybrid file watcher (ADR-0014). Combines two signals into one Flow:
 *
 * 1. A `ContentObserver` registered on the file's URI. Where the SAF
 *    DocumentsProvider supports change notification this gives
 *    sub-second reaction. ExternalStorageProvider's coverage is
 *    inconsistent across OEMs and Android versions, and third-party
 *    writers (e.g. Syncthing) often bypass it — so on its own this
 *    isn't sufficient.
 *
 * 2. A periodic poll that re-stats the file every `pollIntervalMs`
 *    (default 5s) and re-reads it when (mtime, size) drifts from the
 *    last emitted meta. Catches anything the observer missed.
 *
 * Both sources funnel through one internal `checkAndEmit` which
 * dedupes against the last-emitted meta, so a real change emits at
 * most once. The poll interval should be paired with
 * `Lifecycle.repeatOnLifecycle(STARTED)` in the caller so neither
 * source runs while the screen is backgrounded.
 *
 * The Flow takes ownership of unregistering the observer and
 * cancelling the poll task on collector cancellation.
 */
fun watchFile(
	context: Context,
	file: DocumentFile,
	initial: FileMeta,
	pollIntervalMs: Long = 5_000L
): Flow<WatcherEvent> = callbackFlow {
	var lastEmitted = initial

	fun checkAndEmit() {
		val currentMtime = file.lastModified()
		val currentSize = file.length()
		if (currentMtime == lastEmitted.mtime && currentSize == lastEmitted.size) {
			return
		}
		val bytes = readFileContent(context, file) ?: return
		val meta = FileMeta(currentMtime, currentSize)
		lastEmitted = meta
		trySend(WatcherEvent(bytes, meta))
	}

	// ContentObserver.onChange fires on the handler's thread. We use
	// the main looper so the observer registration is on a known
	// thread, but the actual file metadata + read work is hopped to
	// the producer scope so SAF round-trips don't jank the UI.
	val producerScope = this
	val handler = Handler(Looper.getMainLooper())
	val observer = object : ContentObserver(handler) {
		override fun onChange(selfChange: Boolean) {
			producerScope.launch { checkAndEmit() }
		}
	}
	context.contentResolver.registerContentObserver(file.uri, false, observer)

	val pollJob = launch {
		while (isActive) {
			delay(pollIntervalMs)
			checkAndEmit()
		}
	}

	awaitClose {
		context.contentResolver.unregisterContentObserver(observer)
		pollJob.cancel()
	}
}
