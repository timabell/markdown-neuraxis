package co.rustworkshop.markdownneuraxis.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * Banner shown above the editor when the open file has changed on
 * disk while the user is editing (ADR-0014 §6). Offers two recovery
 * actions: drop the in-progress edit and reload the disk version, or
 * write the current buffer to a sibling `.mdnx-conflict-*.md` file
 * before reloading.
 */
@Composable
fun ExternalChangeBanner(
	fileDisplayName: String,
	onDiscard: () -> Unit,
	onSaveCopy: () -> Unit,
	modifier: Modifier = Modifier
) {
	Surface(
		modifier = modifier.fillMaxWidth(),
		color = MaterialTheme.colorScheme.tertiaryContainer,
		tonalElevation = 2.dp
	) {
		Column(
			modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp)
		) {
			Text(
				text = "'$fileDisplayName' changed on disk while you were editing.",
				style = MaterialTheme.typography.bodyMedium,
				color = MaterialTheme.colorScheme.onTertiaryContainer
			)
			Row(
				modifier = Modifier
					.fillMaxWidth()
					.padding(top = 8.dp),
				horizontalArrangement = Arrangement.End,
				verticalAlignment = Alignment.CenterVertically
			) {
				TextButton(onClick = onDiscard) {
					Text("Discard mine")
				}
				Spacer(modifier = Modifier.width(8.dp))
				TextButton(onClick = onSaveCopy) {
					Text("Save copy")
				}
			}
		}
	}
}
