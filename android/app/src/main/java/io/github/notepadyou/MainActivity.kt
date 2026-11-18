package io.github.notepadyou

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.Surface
import androidx.compose.material3.Typography
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.foundation.layout.fillMaxSize
import androidx.core.content.IntentCompat
import androidx.core.view.WindowCompat
import kotlinx.coroutines.launch
import java.io.File

class MainActivity : ComponentActivity() {
    private lateinit var repo: NotesRepo
    private val pendingOpen = mutableStateOf<File?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        repo = NotesRepo(this)
        val prefs = Prefs(this)
        if (!prefs.seeded) {
            repo.seedWelcome()
            prefs.seeded = true
        }
        handleIntent(intent)
        setContent { NotepadApp(repo, prefs, pendingOpen) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    /** Файл .md из другого приложения или «Поделиться» текстом → новая заметка. */
    private fun handleIntent(intent: Intent?) {
        if (intent == null) return
        when (intent.action) {
            Intent.ACTION_VIEW -> intent.data?.let { importUri(it) }
            Intent.ACTION_SEND -> {
                val stream = IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
                if (stream != null) {
                    importUri(stream)
                } else {
                    val text = intent.getStringExtra(Intent.EXTRA_TEXT) ?: return
                    val title = intent.getStringExtra(Intent.EXTRA_SUBJECT) ?: "Из «Поделиться»"
                    pendingOpen.value = repo.create(title, text)
                }
            }
        }
    }

    fun importUri(uri: Uri) {
        val text = runCatching { contentResolver.openInputStream(uri)?.bufferedReader()?.use { it.readText() } }.getOrNull() ?: return
        val name = runCatching {
            contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                if (it.moveToFirst()) it.getString(0) else null
            }
        }.getOrNull()?.substringBeforeLast('.') ?: "Импорт"
        pendingOpen.value = repo.create(name, text)
    }
}

sealed interface Screen {
    data object List : Screen
    data class Edit(val file: File) : Screen
    data object Settings : Screen
}

@Composable
fun NotepadApp(repo: NotesRepo, prefs: Prefs, pendingOpen: MutableState<File?>) {
    val context = LocalContext.current
    val dark = when (prefs.themeMode) {
        1 -> false
        2 -> true
        else -> isSystemInDarkTheme()
    }
    val scheme = if (prefs.dynamic && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
    } else {
        seedScheme(prefs.seed, dark)
    }

    // Цвет значков в строке состояния под нашу тему
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
        }
    }

    val typography = remember(prefs.nunito) { if (prefs.nunito) nunitoTypography() else Typography() }
    MaterialTheme(colorScheme = scheme.animated(), typography = typography) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            AnimatedContent(
                targetState = prefs.onboarded,
                transitionSpec = { (fadeIn() + scaleIn(initialScale = 0.92f)) togetherWith fadeOut() },
                label = "onboarding",
            ) { done ->
                if (!done) Onboarding(onDone = { prefs.onboarded = true })
                else MainNavigation(repo, prefs, pendingOpen)
            }
        }
    }
}

@Composable
private fun MainNavigation(repo: NotesRepo, prefs: Prefs, pendingOpen: MutableState<File?>) {
    var screen by remember { mutableStateOf<Screen>(Screen.List) }
    var notes by remember { mutableStateOf(repo.list()) }
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val activity = LocalContext.current as MainActivity

    LaunchedEffect(pendingOpen.value) {
        pendingOpen.value?.let {
            notes = repo.list()
            screen = Screen.Edit(it)
            pendingOpen.value = null
        }
    }

    fun back() {
        notes = repo.list()
        screen = Screen.List
    }
    BackHandler(enabled = screen != Screen.List) { back() }

    AnimatedContent(
        targetState = screen,
        transitionSpec = {
            if (targetState == Screen.List) {
                (slideInHorizontally { -it / 4 } + fadeIn()) togetherWith (slideOutHorizontally { it / 3 } + fadeOut())
            } else {
                (slideInHorizontally { it / 3 } + fadeIn()) togetherWith (slideOutHorizontally { -it / 4 } + fadeOut())
            }
        },
        label = "screen",
    ) { s ->
        when (s) {
            Screen.List -> NotesScreen(
                notes = notes,
                snackbar = snackbar,
                onOpen = { screen = Screen.Edit(it.file) },
                onNew = {
                    val f = repo.create()
                    notes = repo.list()
                    screen = Screen.Edit(f)
                },
                onDelete = { note ->
                    repo.delete(note.file)
                    notes = repo.list()
                    scope.launch {
                        val r = snackbar.showSnackbar("Заметка «${note.title}» удалена", "Вернуть", duration = SnackbarDuration.Short)
                        if (r == SnackbarResult.ActionPerformed) {
                            note.file.writeText(note.text)
                            notes = repo.list()
                        }
                    }
                },
                onImport = { uri -> activity.importUri(uri) },
                onSettings = { screen = Screen.Settings },
            )
            is Screen.Edit -> EditorScreen(
                file = s.file,
                repo = repo,
                prefs = prefs,
                onBack = { back() },
                onDeleted = { note ->
                    back()
                    scope.launch {
                        val r = snackbar.showSnackbar("Заметка «${note.title}» удалена", "Вернуть", duration = SnackbarDuration.Short)
                        if (r == SnackbarResult.ActionPerformed) {
                            note.file.writeText(note.text)
                            notes = repo.list()
                        }
                    }
                },
            )
            Screen.Settings -> SettingsScreen(prefs, onBack = { back() })
        }
    }
}
