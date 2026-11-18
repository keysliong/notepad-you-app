package io.github.notepadyou

import android.os.Build
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun SettingsScreen(prefs: Prefs, onBack: () -> Unit) {
    val cs = MaterialTheme.colorScheme
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    val canDynamic = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
    val dark = cs.background.luminance() < 0.5f

    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        topBar = {
            LargeTopAppBar(
                title = { Text("Настройки") },
                navigationIcon = { CookieIconButton(Icons.AutoMirrored.Filled.ArrowBack, "Назад", onBack, Modifier.padding(start = 8.dp)) },
                scrollBehavior = scroll,
                colors = TopAppBarDefaults.largeTopAppBarColors(containerColor = cs.background, scrolledContainerColor = cs.surfaceContainer),
            )
        },
        containerColor = cs.background,
    ) { pad ->
        LazyColumn(
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = pad.calculateTopPadding(), bottom = pad.calculateBottomPadding() + 32.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                Section("Цвета") {
                    if (canDynamic) {
                        SwitchRow("Цвета из обоев", "Material You подберёт цвета под обои телефона", prefs.dynamic) { prefs.dynamic = it }
                        Spacer(Modifier.height(8.dp))
                    }
                    Column(Modifier.alpha(if (prefs.dynamic && canDynamic) 0.4f else 1f)) {
                        Text("Цвет оформления", style = MaterialTheme.typography.titleSmall, color = cs.onSurfaceVariant)
                        Spacer(Modifier.height(10.dp))
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            for (seed in Seed.entries) {
                                Swatch(seed, selected = seed == prefs.seed && !(prefs.dynamic && canDynamic), dark = dark) {
                                    prefs.seed = seed
                                    prefs.dynamic = false
                                }
                            }
                        }
                    }
                }
            }
            item {
                Section("Тема") {
                    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                        listOf("Авто", "Светлая", "Тёмная").forEachIndexed { i, label ->
                            SegmentedButton(
                                selected = prefs.themeMode == i,
                                onClick = { prefs.themeMode = i },
                                shape = SegmentedButtonDefaults.itemShape(i, 3),
                            ) { Text(label, maxLines = 1) }
                        }
                    }
                }
            }
            item {
                Section("Текст") {
                    SwitchRow("Шрифт Nunito", "Мягкий скруглённый шрифт, как в Notepad You для ПК", prefs.nunito) { prefs.nunito = it }
                    Spacer(Modifier.height(12.dp))
                    Text("Размер текста: ${prefs.fontSize.toInt()}", style = MaterialTheme.typography.titleSmall, color = cs.onSurfaceVariant)
                    Slider(value = prefs.fontSize, onValueChange = { prefs.fontSize = it }, valueRange = 12f..26f, steps = 13)
                    Surface(shape = RoundedCornerShape(16.dp), color = cs.surfaceContainerHighest) {
                        MarkdownView(
                            text = "**Пример текста** 🍪 — так будут выглядеть заметки",
                            fontSize = prefs.fontSize,
                            onToggleTask = {},
                            modifier = Modifier.padding(14.dp).height(48.dp),
                        )
                    }
                }
            }
            item {
                Section("Приветствие") {
                    OutlinedButton(onClick = { prefs.onboarded = false }) { Text("Показать приветствие ещё раз") }
                }
            }
            item {
                Text(
                    "Notepad You 1.1 для Android · Material You",
                    style = MaterialTheme.typography.labelMedium, color = cs.outline,
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                )
            }
        }
    }
}

private fun Color.luminance(): Float = 0.2126f * red + 0.7152f * green + 0.0722f * blue

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    val cs = MaterialTheme.colorScheme
    Surface(shape = RoundedCornerShape(28.dp), color = cs.surfaceContainerLow, modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(20.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = cs.primary)
            Spacer(Modifier.height(12.dp))
            content()
        }
    }
}

@Composable
private fun SwitchRow(title: String, subtitle: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(16.dp)).clickable { onChange(!checked) }.padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f).padding(end = 12.dp)) {
            Text(title, style = MaterialTheme.typography.bodyLarge)
            Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Switch(checked = checked, onCheckedChange = onChange)
    }
}

/** Печенька выбора цвета: выбранная поворачивается, появляется галочка. */
@Composable
private fun Swatch(seed: Seed, selected: Boolean, dark: Boolean, onClick: () -> Unit) {
    val color = seedScheme(seed, dark).primary
    val onColor = seedScheme(seed, dark).onPrimary
    val rot by animateFloatAsState(if (selected) 30f else 0f, spring(Spring.DampingRatioMediumBouncy), label = "r")
    val scale by animateFloatAsState(if (selected) 1f else 0.86f, spring(Spring.DampingRatioMediumBouncy), label = "s")
    // Нажимать можно и на печеньку, и на подпись под ней
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.clip(RoundedCornerShape(16.dp)).clickable(onClick = onClick).padding(2.dp),
    ) {
        Box(
            Modifier
                .size(56.dp)
                .graphicsLayer { rotationZ = rot; scaleX = scale; scaleY = scale }
                .clip(CookieShape(7, 0.1f))
                .background(color)
                .then(if (selected) Modifier.border(3.dp, MaterialTheme.colorScheme.onSurface, CookieShape(7, 0.1f)) else Modifier),
            contentAlignment = Alignment.Center,
        ) {
            val check by animateFloatAsState(if (selected) 1f else 0f, spring(Spring.DampingRatioMediumBouncy), label = "c")
            Icon(
                Icons.Filled.Check, null, tint = onColor,
                modifier = Modifier.graphicsLayer { rotationZ = -rot; alpha = check; scaleX = check; scaleY = check },
            )
        }
        Text(seed.title, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
