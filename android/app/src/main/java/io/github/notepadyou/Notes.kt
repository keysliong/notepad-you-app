package io.github.notepadyou

import android.content.Context
import java.io.File

data class Note(val file: File, val title: String, val text: String, val modified: Long)

/** Заметки — обычные .md файлы во внутренней памяти приложения. */
class NotesRepo(context: Context) {
    private val dir = File(context.filesDir, "notes").apply { mkdirs() }

    fun list(): List<Note> =
        (dir.listFiles { f -> f.isFile && f.extension.lowercase() in setOf("md", "txt", "markdown") } ?: emptyArray())
            .map { Note(it, it.nameWithoutExtension, runCatching { it.readText() }.getOrDefault(""), it.lastModified()) }
            .sortedByDescending { it.modified }

    private fun clean(title: String): String =
        title.replace(Regex("[\\\\/:*?\"<>|\\n\\r\\t]"), "_").trim().take(60).ifEmpty { "Заметка" }

    private fun unique(base: String, except: File? = null): File {
        var f = File(dir, "$base.md")
        var n = 2
        while (f.exists() && f != except) {
            f = File(dir, "$base $n.md")
            n++
        }
        return f
    }

    fun create(title: String = "Новая заметка", text: String = ""): File {
        val f = unique(clean(title))
        f.writeText(text)
        return f
    }

    fun save(file: File, text: String) {
        if (file.exists() && runCatching { file.readText() }.getOrNull() == text) return
        file.writeText(text)
    }

    fun rename(file: File, title: String): File {
        val base = clean(title)
        if (base == file.nameWithoutExtension) return file
        val target = unique(base, except = file)
        return if (file.renameTo(target)) target else file
    }

    fun delete(file: File) {
        file.delete()
    }

    /** Первый запуск: приветственная заметка. */
    fun seedWelcome() {
        if (list().isEmpty()) create("Добро пожаловать", WELCOME)
    }
}

const val WELCOME = """# Добро пожаловать в Notepad You 👋

Красивый блокнот в стиле **Material You** с поддержкой *Markdown*. Смайлики тоже цветные 🍪✨🎨

## Что умеет
- Редактор с подсветкой разметки
- **Предпросмотр** — переключатель вверху экрана
- Цвета из обоев телефона или 7 своих тем
- Заметки сохраняются сами

## Задачи можно отмечать пальцем
- [x] Установить Notepad You 🎉
- [ ] Написать первую заметку ✍️
- [ ] Выбрать любимый цвет в настройках 🎨

> Совет: смахните заметку в списке в сторону, чтобы удалить её.

```
fun main() = println("Привет, мир!")
```
"""
