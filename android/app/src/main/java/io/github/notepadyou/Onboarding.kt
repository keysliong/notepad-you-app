package io.github.notepadyou

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import kotlin.math.absoluteValue

private data class Page(val emoji: String?, val title: String, val text: String)

private val PAGES = listOf(
    Page(null, "Добро пожаловать в Notepad You", "Красивый блокнот в стиле Material You. Заметки сохраняются сами — просто пишите."),
    Page("📝", "Markdown и предпросмотр", "Заголовки, списки, задачи и код. Переключайтесь между «Редактором» и «Просмотром», а галочки отмечайте пальцем."),
    Page("🎨", "Сделайте по-своему", "Цвета из обоев телефона или одна из 7 тем-печенек. Светлая и тёмная тема — в настройках."),
)

/** Приветствие при первом запуске: три страницы с крутящимися печеньками. */
@Composable
fun Onboarding(onDone: () -> Unit) {
    val cs = MaterialTheme.colorScheme
    val pager = rememberPagerState { PAGES.size }
    val scope = rememberCoroutineScope()
    val last = pager.currentPage == PAGES.lastIndex

    Column(Modifier.fillMaxSize().statusBarsPadding().navigationBarsPadding().padding(24.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            TextButton(onClick = onDone) { Text("Пропустить") }
        }
        HorizontalPager(state = pager, modifier = Modifier.weight(1f)) { index ->
            val page = PAGES[index]
            // Параллакс: страница слегка уменьшается и тускнеет при перелистывании
            val offset = ((pager.currentPage - index) + pager.currentPageOffsetFraction).absoluteValue.coerceIn(0f, 1f)
            Column(
                Modifier.fillMaxSize().graphicsLayer {
                    alpha = 1f - offset * 0.6f
                    scaleX = 1f - offset * 0.15f
                    scaleY = 1f - offset * 0.15f
                },
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                SpinningCookie(220.dp, if (index == 1) cs.tertiaryContainer else cs.primaryContainer, lobes = 12 - index * 2, periodMs = 16000 + index * 4000) {
                    if (page.emoji == null) {
                        Image(painterResource(R.drawable.logo), null, Modifier.size(150.dp))
                    } else {
                        Text(page.emoji, fontSize = 84.sp)
                    }
                }
                Spacer(Modifier.height(40.dp))
                Text(page.title, style = MaterialTheme.typography.headlineMedium, textAlign = TextAlign.Center)
                Spacer(Modifier.height(12.dp))
                Text(page.text, style = MaterialTheme.typography.bodyLarge, color = cs.onSurfaceVariant, textAlign = TextAlign.Center)
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = 16.dp), verticalAlignment = Alignment.CenterVertically) {
            // Точки-индикаторы: текущая вытягивается в «таблетку»
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                repeat(PAGES.size) { i ->
                    val selected = i == pager.currentPage
                    val width by animateDpAsState(if (selected) 28.dp else 8.dp, label = "w")
                    val color by animateColorAsState(if (selected) cs.primary else cs.outlineVariant, label = "c")
                    Box(Modifier.height(8.dp).width(width).clip(CircleShape).background(color))
                }
            }
            Spacer(Modifier.weight(1f))
            Button(onClick = { if (last) onDone() else scope.launch { pager.animateScrollToPage(pager.currentPage + 1) } }) {
                AnimatedContent(last, transitionSpec = { fadeIn() togetherWith fadeOut() }, label = "btn") {
                    Text(if (it) "Начать 🍪" else "Далее")
                }
            }
        }
    }
}
