package io.github.notepadyou

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeFloatingActionButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

/** Фигура «печенька» (Cookie из Material 3 Expressive): круг с волнистым краем. */
class CookieShape(
    private val lobes: Int = 9,
    private val depth: Float = 0.09f,
    private val rotation: Float = 0f,
) : Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density): Outline {
        val path = Path()
        val cx = size.width / 2f
        val cy = size.height / 2f
        val r = min(cx, cy)
        val n = lobes * 16
        for (i in 0..n) {
            val a = 2.0 * PI * i / n
            val wave = 0.5f + 0.5f * cos(lobes * (a - rotation)).toFloat()
            val rr = r * (1f - depth + depth * wave)
            val x = cx + cos(a).toFloat() * rr
            val y = cy + sin(a).toFloat() * rr
            if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        path.close()
        return Outline.Generic(path)
    }
}

/** Кнопка-иконка в форме печеньки: при нажатии сжимается и проворачивается. */
@Composable
fun CookieIconButton(
    icon: ImageVector,
    description: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    container: Color = MaterialTheme.colorScheme.surfaceContainerHigh,
    content: Color = MaterialTheme.colorScheme.onSurfaceVariant,
) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.86f else 1f, spring(Spring.DampingRatioMediumBouncy), label = "s")
    val rot by animateFloatAsState(if (pressed) 40f else 0f, spring(Spring.DampingRatioLowBouncy), label = "r")
    Box(
        modifier
            .size(44.dp)
            .graphicsLayer { scaleX = scale; scaleY = scale; rotationZ = rot }
            .clip(CookieShape(9, 0.09f))
            .background(container)
            .clickable(interactionSource = source, indication = LocalIndication.current, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, description, tint = content, modifier = Modifier.size(22.dp).graphicsLayer { rotationZ = -rot })
    }
}

/** Кнопка панели форматирования (текстовая метка в печеньке). */
@Composable
fun CookieToolButton(label: String, onClick: () -> Unit) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.82f else 1f, spring(Spring.DampingRatioMediumBouncy), label = "s")
    val rot by animateFloatAsState(if (pressed) 35f else 0f, spring(Spring.DampingRatioLowBouncy), label = "r")
    val cs = MaterialTheme.colorScheme
    Box(
        Modifier
            .size(46.dp)
            .graphicsLayer { scaleX = scale; scaleY = scale; rotationZ = rot }
            .clip(CookieShape(6, 0.11f))
            .background(if (pressed) cs.primaryContainer else cs.surfaceContainerHighest)
            .clickable(interactionSource = source, indication = LocalIndication.current, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            color = if (pressed) cs.onPrimaryContainer else cs.onSurfaceVariant,
            fontWeight = FontWeight.Bold,
            fontSize = 15.sp,
            modifier = Modifier.graphicsLayer { rotationZ = -rot },
        )
    }
}

/** Большая FAB-печенька: медленно вращается, при нажатии «пружинит». */
@Composable
fun CookieFab(onClick: () -> Unit, modifier: Modifier = Modifier) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val spin by rememberInfiniteTransition(label = "spin").animateFloat(
        0f, (2 * PI).toFloat(), infiniteRepeatable(tween(24000, easing = LinearEasing)), label = "a",
    )
    val scale by animateFloatAsState(if (pressed) 0.88f else 1f, spring(Spring.DampingRatioMediumBouncy), label = "s")
    LargeFloatingActionButton(
        onClick = onClick,
        shape = CookieShape(9, 0.09f, spin),
        containerColor = MaterialTheme.colorScheme.primaryContainer,
        contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
        interactionSource = source,
        modifier = modifier.graphicsLayer { scaleX = scale; scaleY = scale },
    ) {
        Icon(Icons.Filled.Add, "Новая заметка", Modifier.size(36.dp))
    }
}

/** Вращающаяся печенька-иллюстрация (пустой список, приветствие). */
@Composable
fun SpinningCookie(
    size: androidx.compose.ui.unit.Dp,
    color: Color,
    modifier: Modifier = Modifier,
    lobes: Int = 12,
    periodMs: Int = 18000,
    content: @Composable () -> Unit = {},
) {
    val spin by rememberInfiniteTransition(label = "c").animateFloat(
        0f, (2 * PI).toFloat(), infiniteRepeatable(tween(periodMs, easing = LinearEasing)), label = "a",
    )
    Box(modifier.size(size), contentAlignment = Alignment.Center) {
        Box(Modifier.size(size).clip(CookieShape(lobes, 0.08f, spin)).background(color))
        content()
    }
}
