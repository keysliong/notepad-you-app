package io.github.notepadyou

import android.net.Uri
import android.text.format.DateUtils
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberSwipeToDismissBoxState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NotesScreen(
    notes: List<Note>,
    snackbar: SnackbarHostState,
    onOpen: (Note) -> Unit,
    onNew: () -> Unit,
    onDelete: (Note) -> Unit,
    onImport: (Uri) -> Unit,
    onSettings: () -> Unit,
) {
    val cs = MaterialTheme.colorScheme
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior()
    var menu by remember { mutableStateOf(false) }
    val openFile = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(onImport) }

    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        topBar = {
            LargeTopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Image(painterResource(R.drawable.logo), null, Modifier.size(40.dp))
                        Spacer(Modifier.width(10.dp))
                        Text("Notepad You", fontWeight = FontWeight.Bold, color = cs.primary)
                    }
                },
                actions = {
                    CookieIconButton(Icons.Filled.Settings, "Настройки", onSettings)
                    Spacer(Modifier.width(6.dp))
                    Box {
                        CookieIconButton(Icons.Filled.MoreVert, "Ещё", { menu = true })
                        DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                            DropdownMenuItem(text = { Text("Открыть файл…") }, onClick = {
                                menu = false
                                openFile.launch(arrayOf("text/markdown", "text/x-markdown", "text/plain"))
                            })
                        }
                    }
                    Spacer(Modifier.width(8.dp))
                },
                scrollBehavior = scroll,
                colors = TopAppBarDefaults.largeTopAppBarColors(containerColor = cs.background, scrolledContainerColor = cs.surfaceContainer),
            )
        },
        floatingActionButton = { CookieFab(onNew) },
        snackbarHost = { SnackbarHost(snackbar) },
        containerColor = cs.background,
    ) { pad ->
        if (notes.isEmpty()) {
            EmptyState(Modifier.padding(pad))
        } else {
            LazyColumn(
                contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = pad.calculateTopPadding() + 4.dp, bottom = pad.calculateBottomPadding() + 120.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(notes, key = { it.file.path }) { note ->
                    SwipeNote(note, onOpen, onDelete, Modifier.animateItem())
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SwipeNote(note: Note, onOpen: (Note) -> Unit, onDelete: (Note) -> Unit, modifier: Modifier) {
    val cs = MaterialTheme.colorScheme
    val state = rememberSwipeToDismissBoxState(confirmValueChange = {
        if (it != SwipeToDismissBoxValue.Settled) {
            onDelete(note)
            true
        } else {
            false
        }
    })
    SwipeToDismissBox(
        state = state,
        modifier = modifier,
        backgroundContent = {
            val start = state.dismissDirection == SwipeToDismissBoxValue.StartToEnd
            Box(
                Modifier.fillMaxSize().clip(RoundedCornerShape(28.dp)).background(cs.errorContainer).padding(horizontal = 28.dp),
                contentAlignment = if (start) Alignment.CenterStart else Alignment.CenterEnd,
            ) {
                Icon(Icons.Filled.Delete, "Удалить", tint = cs.onErrorContainer)
            }
        },
    ) {
        NoteCard(note) { onOpen(note) }
    }
}

@Composable
private fun NoteCard(note: Note, onClick: () -> Unit) {
    val cs = MaterialTheme.colorScheme
    Surface(onClick = onClick, shape = RoundedCornerShape(28.dp), color = cs.surfaceContainerLow, modifier = Modifier.fillMaxWidth()) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            // Печенька с первой буквой или смайликом из заметки
            val badge = remember(note.text, note.title) { badgeFor(note) }
            Box(Modifier.size(48.dp).clip(CookieShape(8, 0.1f)).background(cs.primaryContainer), contentAlignment = Alignment.Center) {
                Text(badge, color = cs.onPrimaryContainer, fontWeight = FontWeight.Bold, fontSize = 20.sp)
            }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(note.title, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                val preview = remember(note.text) { previewOf(note) }
                if (preview.isNotBlank()) {
                    Text(preview, style = MaterialTheme.typography.bodyMedium, color = cs.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
                Spacer(Modifier.height(4.dp))
                Text(
                    DateUtils.getRelativeTimeSpanString(note.modified).toString(),
                    style = MaterialTheme.typography.labelSmall, color = cs.outline,
                )
            }
        }
    }
}

private fun badgeFor(note: Note): String {
    // Первый смайлик из текста, иначе первая буква названия
    val emoji = Regex("[\\x{1F300}-\\x{1FAFF}\\x{2600}-\\x{27BF}]").find(note.title + " " + note.text)
    return emoji?.value ?: note.title.firstOrNull { it.isLetterOrDigit() }?.uppercase() ?: "✎"
}

@Composable
private fun EmptyState(modifier: Modifier) {
    val cs = MaterialTheme.colorScheme
    Column(modifier.fillMaxSize().padding(32.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        SpinningCookie(160.dp, cs.primaryContainer) {
            Text("📝", fontSize = 56.sp)
        }
        Spacer(Modifier.height(24.dp))
        Text("Пока пусто", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(6.dp))
        Text(
            "Нажмите на печеньку внизу, чтобы создать первую заметку",
            style = MaterialTheme.typography.bodyLarge, color = cs.onSurfaceVariant, textAlign = TextAlign.Center,
        )
    }
}
