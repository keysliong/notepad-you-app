package io.github.notepadyou

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.tween
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import kotlin.math.abs
import kotlin.math.min

/** Те же 7 цветов, что и в Notepad You для ПК. */
enum class Seed(val title: String, val hue: Float, val sat: Float) {
    Lavender("Лаванда", 262f, 0.42f),
    Coral("Коралл", 12f, 0.78f),
    Ocean("Океан", 208f, 0.55f),
    Mint("Мята", 158f, 0.42f),
    Rose("Роза", 340f, 0.50f),
    Amber("Янтарь", 30f, 0.62f),
    Graphite("Графит", 220f, 0.10f),
}

private fun hsl(h: Float, s: Float, l: Float): Color {
    val c = (1f - abs(2f * l - 1f)) * s
    val hp = (h % 360f) / 60f
    val x = c * (1f - abs(hp % 2f - 1f))
    val (r, g, b) = when (hp.toInt()) {
        0 -> Triple(c, x, 0f)
        1 -> Triple(x, c, 0f)
        2 -> Triple(0f, c, x)
        3 -> Triple(0f, x, c)
        4 -> Triple(x, 0f, c)
        else -> Triple(c, 0f, x)
    }
    val m = l - c / 2f
    return Color((r + m).coerceIn(0f, 1f), (g + m).coerceIn(0f, 1f), (b + m).coerceIn(0f, 1f))
}

private fun tone(h: Float, s: Float, t: Float) = hsl(h, min(s, 1f), (t / 100f).coerceIn(0f, 1f))

/** Тональная палитра Material 3 из «цвета-семени» (как theme.rs на ПК). */
fun seedScheme(seed: Seed, dark: Boolean): ColorScheme {
    val h = seed.hue
    val s = seed.sat
    fun p(t: Float) = tone(h, s, t)
    fun sec(t: Float) = tone(h, s * 0.35f, t)
    fun ter(t: Float) = tone((h + 60f) % 360f, s * 0.6f, t)
    fun n(t: Float) = tone(h, min(s, 0.08f), t)
    fun nv(t: Float) = tone(h, min(s * 0.25f, 0.14f), t)
    return if (dark) darkColorScheme(
        primary = p(80f), onPrimary = p(20f), primaryContainer = p(30f), onPrimaryContainer = p(90f),
        inversePrimary = p(40f),
        secondary = sec(80f), onSecondary = sec(20f), secondaryContainer = sec(30f), onSecondaryContainer = sec(90f),
        tertiary = ter(80f), onTertiary = ter(20f), tertiaryContainer = ter(30f), onTertiaryContainer = ter(90f),
        background = n(6f), onBackground = n(90f), surface = n(6f), onSurface = n(90f),
        surfaceVariant = nv(30f), onSurfaceVariant = nv(80f), outline = nv(60f), outlineVariant = nv(30f),
        inverseSurface = n(90f), inverseOnSurface = n(20f), surfaceTint = p(80f),
        surfaceContainerLowest = n(4f), surfaceContainerLow = n(10f), surfaceContainer = n(12f),
        surfaceContainerHigh = n(17f), surfaceContainerHighest = n(22f),
        surfaceBright = n(24f), surfaceDim = n(6f),
    ) else lightColorScheme(
        primary = p(40f), onPrimary = p(100f), primaryContainer = p(90f), onPrimaryContainer = p(10f),
        inversePrimary = p(80f),
        secondary = sec(40f), onSecondary = sec(100f), secondaryContainer = sec(90f), onSecondaryContainer = sec(10f),
        tertiary = ter(40f), onTertiary = ter(100f), tertiaryContainer = ter(90f), onTertiaryContainer = ter(10f),
        background = n(98f), onBackground = n(10f), surface = n(98f), onSurface = n(10f),
        surfaceVariant = nv(90f), onSurfaceVariant = nv(30f), outline = nv(50f), outlineVariant = nv(80f),
        inverseSurface = n(20f), inverseOnSurface = n(95f), surfaceTint = p(40f),
        surfaceContainerLowest = n(100f), surfaceContainerLow = n(96f), surfaceContainer = n(94f),
        surfaceContainerHigh = n(92f), surfaceContainerHighest = n(89f),
        surfaceBright = n(98f), surfaceDim = n(87f),
    )
}

/** Плавный переход между палитрами при смене темы. */
@Composable
fun ColorScheme.animated(): ColorScheme {
    val spec = tween<Color>(durationMillis = 450)
    @Composable
    fun a(c: Color): Color = animateColorAsState(c, spec, label = "color").value
    return copy(
        primary = a(primary), onPrimary = a(onPrimary),
        primaryContainer = a(primaryContainer), onPrimaryContainer = a(onPrimaryContainer),
        secondary = a(secondary), onSecondary = a(onSecondary),
        secondaryContainer = a(secondaryContainer), onSecondaryContainer = a(onSecondaryContainer),
        tertiary = a(tertiary), tertiaryContainer = a(tertiaryContainer), onTertiaryContainer = a(onTertiaryContainer),
        background = a(background), onBackground = a(onBackground),
        surface = a(surface), onSurface = a(onSurface),
        surfaceVariant = a(surfaceVariant), onSurfaceVariant = a(onSurfaceVariant),
        outline = a(outline), outlineVariant = a(outlineVariant),
        surfaceContainerLow = a(surfaceContainerLow), surfaceContainer = a(surfaceContainer),
        surfaceContainerHigh = a(surfaceContainerHigh), surfaceContainerHighest = a(surfaceContainerHighest),
        errorContainer = a(errorContainer), onErrorContainer = a(onErrorContainer),
    )
}

val Nunito = FontFamily(
    Font(R.font.nunito_regular, FontWeight.Normal),
    Font(R.font.nunito_bold, FontWeight.Bold),
)

/** Типографика Material 3 со шрифтом Nunito (как в Notepad You для ПК). */
fun nunitoTypography(): Typography {
    val d = Typography()
    return Typography(
        displayLarge = d.displayLarge.copy(fontFamily = Nunito),
        displayMedium = d.displayMedium.copy(fontFamily = Nunito),
        displaySmall = d.displaySmall.copy(fontFamily = Nunito),
        headlineLarge = d.headlineLarge.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        headlineMedium = d.headlineMedium.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        headlineSmall = d.headlineSmall.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        titleLarge = d.titleLarge.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        titleMedium = d.titleMedium.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        titleSmall = d.titleSmall.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        bodyLarge = d.bodyLarge.copy(fontFamily = Nunito),
        bodyMedium = d.bodyMedium.copy(fontFamily = Nunito),
        bodySmall = d.bodySmall.copy(fontFamily = Nunito),
        labelLarge = d.labelLarge.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        labelMedium = d.labelMedium.copy(fontFamily = Nunito, fontWeight = FontWeight.Bold),
        labelSmall = d.labelSmall.copy(fontFamily = Nunito),
    )
}
