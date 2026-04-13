package co.rustworkshop.markdownneuraxis.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.*
import androidx.compose.runtime.Composable

@Composable
fun AppBottomBar(
    onMenuClick: () -> Unit,
    onHomeClick: () -> Unit,
    isEditing: Boolean = false,
    onDoneClick: (() -> Unit)? = null
) {
    BottomAppBar(
        actions = {
            IconButton(onClick = onMenuClick) {
                Icon(Icons.Default.Menu, contentDescription = "Menu")
            }
            IconButton(onClick = onHomeClick) {
                Icon(Icons.Default.Home, contentDescription = "Home")
            }
        },
        floatingActionButton = if (isEditing && onDoneClick != null) {
            {
                FloatingActionButton(
                    onClick = onDoneClick,
                    containerColor = BottomAppBarDefaults.bottomAppBarFabColor,
                    elevation = FloatingActionButtonDefaults.bottomAppBarFabElevation()
                ) {
                    Icon(Icons.Default.Check, contentDescription = "Done editing")
                }
            }
        } else null
    )
}
