package co.rustworkshop.markdownneuraxis.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Solarized color palette
private val Base03 = Color(0xFF002b36)  // Dark background
private val Base02 = Color(0xFF073642)  // Dark background highlight
private val Base01 = Color(0xFF586e75)  // Dark content / Light emphasized content
private val Base00 = Color(0xFF657b83)  // Dark content / Light content
private val Base0 = Color(0xFF839496)   // Light content / Dark content
private val Base1 = Color(0xFF93a1a1)   // Light emphasized content / Dark content
private val Base2 = Color(0xFFeee8d5)   // Light background highlight
private val Base3 = Color(0xFFfdf6e3)   // Light background

private val Yellow = Color(0xFFb58900)
private val Orange = Color(0xFFcb4b16)
private val Red = Color(0xFFdc322f)
private val Magenta = Color(0xFFd33682)
private val Violet = Color(0xFF6c71c4)
private val Blue = Color(0xFF268bd2)
private val Cyan = Color(0xFF2aa198)
private val Green = Color(0xFF859900)

private val SolarizedLightColorScheme = lightColorScheme(
    primary = Blue,
    onPrimary = Base3,
    primaryContainer = Base2,
    onPrimaryContainer = Blue,
    secondary = Cyan,
    onSecondary = Base3,
    secondaryContainer = Base2,
    onSecondaryContainer = Cyan,
    tertiary = Violet,
    onTertiary = Base3,
    tertiaryContainer = Base2,
    onTertiaryContainer = Violet,
    error = Red,
    onError = Base3,
    errorContainer = Base2,
    onErrorContainer = Red,
    background = Base3,
    onBackground = Base00,
    surface = Base3,
    onSurface = Base00,
    surfaceVariant = Base2,
    onSurfaceVariant = Base01,
    outline = Base1,
    outlineVariant = Base2,
    inverseSurface = Base02,
    inverseOnSurface = Base1,
    inversePrimary = Cyan
)

private val SolarizedDarkColorScheme = darkColorScheme(
    primary = Blue,
    onPrimary = Base03,
    primaryContainer = Base02,
    onPrimaryContainer = Blue,
    secondary = Cyan,
    onSecondary = Base03,
    secondaryContainer = Base02,
    onSecondaryContainer = Cyan,
    tertiary = Violet,
    onTertiary = Base03,
    tertiaryContainer = Base02,
    onTertiaryContainer = Violet,
    error = Red,
    onError = Base03,
    errorContainer = Base02,
    onErrorContainer = Red,
    background = Base03,
    onBackground = Base0,
    surface = Base03,
    onSurface = Base0,
    surfaceVariant = Base02,
    onSurfaceVariant = Base1,
    outline = Base01,
    outlineVariant = Base02,
    inverseSurface = Base2,
    inverseOnSurface = Base01,
    inversePrimary = Blue
)

@Composable
fun MarkdownNeuraxisTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) SolarizedDarkColorScheme else SolarizedLightColorScheme

    MaterialTheme(
        colorScheme = colorScheme,
        content = content
    )
}
