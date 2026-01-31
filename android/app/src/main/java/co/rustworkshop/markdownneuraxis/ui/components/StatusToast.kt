package co.rustworkshop.markdownneuraxis.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * Reusable status toast overlay for showing transient status messages.
 * Appears as a small pill in the corner without affecting layout.
 */
@Composable
fun StatusToast(
    message: String,
    modifier: Modifier = Modifier,
    showProgress: Boolean = false
) {
    Surface(
        modifier = modifier.padding(8.dp),
        shape = MaterialTheme.shapes.small,
        color = MaterialTheme.colorScheme.surfaceVariant,
        tonalElevation = 2.dp,
        shadowElevation = 4.dp
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            if (showProgress) {
                CircularProgressIndicator(
                    modifier = Modifier.size(14.dp),
                    strokeWidth = 2.dp
                )
                Spacer(modifier = Modifier.width(8.dp))
            }
            Text(
                text = message,
                style = MaterialTheme.typography.bodySmall
            )
        }
    }
}
