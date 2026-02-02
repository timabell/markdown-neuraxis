package co.rustworkshop.markdownneuraxis.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.*
import androidx.compose.runtime.Composable

@Composable
fun AppBottomBar(
    onMenuClick: () -> Unit,
    onHomeClick: () -> Unit
) {
    BottomAppBar {
        IconButton(onClick = onMenuClick) {
            Icon(Icons.Default.Menu, contentDescription = "Menu")
        }
        IconButton(onClick = onHomeClick) {
            Icon(Icons.Default.Home, contentDescription = "Home")
        }
    }
}
