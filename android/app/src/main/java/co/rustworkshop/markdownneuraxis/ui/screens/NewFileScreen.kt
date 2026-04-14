package co.rustworkshop.markdownneuraxis.ui.screens

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp

/**
 * Screen for creating a new file. Shows a text editor for content.
 * The file is not saved until the user taps Done in the bottom bar.
 */
@Composable
fun NewFileScreen(
	initialContent: String,
	onContentChanged: (String) -> Unit,
	modifier: Modifier = Modifier,
	autoFocus: Boolean = false
) {
	var textFieldValue by remember {
		mutableStateOf(TextFieldValue(initialContent, TextRange(initialContent.length)))
	}
	val focusRequester = remember { FocusRequester() }

	// Update parent state when content changes
	LaunchedEffect(textFieldValue.text) {
		onContentChanged(textFieldValue.text)
	}

	// Auto-focus when requested
	if (autoFocus) {
		LaunchedEffect(Unit) {
			focusRequester.requestFocus()
		}
	}

	Box(modifier = modifier.fillMaxSize().padding(16.dp)) {
		BasicTextField(
			value = textFieldValue,
			onValueChange = { textFieldValue = it },
			modifier = Modifier.fillMaxSize().focusRequester(focusRequester),
			textStyle = MaterialTheme.typography.bodyLarge.copy(
				color = LocalContentColor.current
			)
		)
		if (textFieldValue.text.isEmpty()) {
			Text(
				text = "Type markdown here...",
				style = MaterialTheme.typography.bodyLarge.copy(
					color = LocalContentColor.current.copy(alpha = 0.5f)
				)
			)
		}
	}
}
