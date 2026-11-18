package io.github.notepadyou

import android.content.Intent
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import java.io.File

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EditorScreen(file: File, repo: NotesRepo, prefs: Prefs, onBack: () -> Unit, onDeleted: (Note) -> Unit) {
    val cs = MaterialTheme.colorScheme
    val context = LocalContext.current
    val focus = LocalFocusManager.current
    var current by remember { mutableStateOf(file) }
    var value by remember { mutableStateOf(TextFieldValue(runCatching { file.readText() }.getOrDefault(""))) }
    var title by remember { mutableStateOf(file.nameWithoutExtension) }
    var preview by rememberSaveable { mutableStateOf(false) }
    var menu by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    var deleted by remember { mutableStateOf(false) }

    // Автосохранение: через полсекунды после последнего изменения
    LaunchedEffect(value.text) {
        delay(500)
        if (!deleted) repo.save(current, value.text)
    }
    fun commitTitle() {
        if (!deleted) current = repo.rename(current, title)
        title = current.nameWithoutExtension
    }
    DisposableEffect(Unit) {
        onDispose {
            if (!deleted) {
                repo.save(current, value.text)
                repo.rename(current, title)
            }
        }
    }

    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/markdown")) { uri ->
        if (uri != null) {
            runCatching { context.contentResolver.openOutputStream(uri)?.use { it.write(value.text.toByteArray()) } }
                .onSuccess { Toast.makeText(context, "Сохранено 💾", Toast.LENGTH_SHORT).show() }
                .onFailure { Toast.makeText(context, "Не удалось сохранить: ${it.message}", Toast.LENGTH_LONG).show() }
        }
    }

    Scaffold(
        topBar = {
            Column {
                TopAppBar(
                    navigationIcon = {
                        CookieIconButton(Icons.AutoMirrored.Filled.ArrowBack, "Назад", onBack, Modifier.padding(start = 8.dp))
                    },
                    title = {
                        BasicTextField(
                            value = title,
                            onValueChange = { title = it },
                            singleLine = true,
                            textStyle = MaterialTheme.typography.titleLarge.copy(color = cs.onSurface),
                            cursorBrush = SolidColor(cs.primary),
                            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = { commitTitle(); focus.clearFocus() }),
                            modifier = Modifier.fillMaxWidth().padding(start = 10.dp),
                        )
                    },
                    actions = {
                        CookieIconButton(Icons.Filled.Share, "Поделиться", {
                            val send = Intent(Intent.ACTION_SEND).setType("text/plain")
                                .putExtra(Intent.EXTRA_SUBJECT, title)
                                .putExtra(Intent.EXTRA_TEXT, value.text)
                            context.startActivity(Intent.createChooser(send, "Поделиться заметкой"))
                        })
                        Spacer(Modifier.width(6.dp))
                        Box {
                            CookieIconButton(Icons.Filled.MoreVert, "Ещё", { menu = true })
                            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                                DropdownMenuItem(text = { Text("Сохранить как файл .md…") }, onClick = {
                                    menu = false
                                    export.launch("$title.md")
                                })
                                DropdownMenuItem(
                                    text = { Text("Удалить заметку") },
                                    leadingIcon = { Icon(Icons.Filled.Delete, null) },
                                    onClick = { menu = false; confirmDelete = true },
                                )
                            }
                        }
                        Spacer(Modifier.width(8.dp))
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = cs.background),
                )
                // Редактор / Просмотр
                SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)) {
                    SegmentedButton(
                        selected = !preview, onClick = { preview = false },
                        shape = SegmentedButtonDefaults.itemShape(0, 2),
                    ) { Text("✏️  Редактор") }
                    SegmentedButton(
                        selected = preview, onClick = { preview = true; focus.clearFocus() },
                        shape = SegmentedButtonDefaults.itemShape(1, 2),
                    ) { Text("👁  Просмотр") }
                }
            }
        },
        bottomBar = {
            AnimatedVisibility(
                visible = !preview,
                enter = slideInVertically { it } + fadeIn(),
                exit = slideOutVertically { it } + fadeOut(),
            ) {
                // Панель форматирования над клавиатурой
                Surface(color = cs.surfaceContainer, modifier = Modifier.navigationBarsPadding().imePadding()) {
                    Row(
                        Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 10.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        for ((label, fmt) in TOOLS) {
                            CookieToolButton(label) { value = fmt.applyTo(value) }
                        }
                    }
                }
            }
        },
        containerColor = cs.background,
    ) { pad ->
        AnimatedContent(
            targetState = preview,
            transitionSpec = { fadeIn() togetherWith fadeOut() },
            modifier = Modifier.padding(pad).fillMaxSize(),
            label = "mode",
        ) { isPreview ->
            if (isPreview) {
                MarkdownView(
                    text = value.text,
                    fontSize = prefs.fontSize,
                    onToggleTask = { line -> value = value.copy(text = toggleTask(value.text, line)) },
                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp),
                )
            } else {
                val highlight = remember(cs) { MarkdownHighlight(cs) }
                val style = TextStyle(
                    fontSize = prefs.fontSize.sp,
                    lineHeight = (prefs.fontSize * 1.5f).sp,
                    color = cs.onSurface,
                    fontFamily = if (prefs.nunito) Nunito else FontFamily.Default,
                )
                BasicTextField(
                    value = value,
                    onValueChange = { value = it },
                    textStyle = style,
                    cursorBrush = SolidColor(cs.primary),
                    visualTransformation = highlight,
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                    modifier = Modifier.fillMaxSize().padding(horizontal = 20.dp, vertical = 12.dp),
                    decorationBox = { inner ->
                        Box {
                            if (value.text.isEmpty()) {
                                Text("Начните писать… Поддерживается Markdown", style = style.copy(color = cs.outline))
                            }
                            inner()
                        }
                    },
                )
            }
        }
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            icon = { Icon(Icons.Filled.Delete, null) },
            title = { Text("Удалить заметку?") },
            text = { Text("«$title» исчезнет из списка. Сразу после удаления её можно вернуть.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    deleted = true
                    val note = Note(current, title, value.text, System.currentTimeMillis())
                    repo.delete(current)
                    onDeleted(note)
                }) { Text("Удалить", fontWeight = FontWeight.Bold) }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text("Отмена") } },
        )
    }
}
