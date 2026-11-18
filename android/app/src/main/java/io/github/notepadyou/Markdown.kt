package io.github.notepadyou

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// ------------------------------------------------------------------ разбор

sealed interface Block {
    data class Heading(val level: Int, val text: String) : Block
    data class Para(val text: String) : Block
    data class Bullet(val text: String, val indent: Int) : Block
    data class Ordered(val number: String, val text: String) : Block
    data class Task(val checked: Boolean, val text: String, val line: Int) : Block
    data class Quote(val text: String) : Block
    data class Code(val text: String) : Block
    data class Table(val rows: List<List<String>>) : Block
    data object Rule : Block
    data object Space : Block
}

private val TASK = Regex("^(\\s*)[-*+] \\[([ xX])] (.*)$")
private val BULLET = Regex("^(\\s*)[-*+] (.*)$")
private val ORDERED = Regex("^\\s*(\\d+)[.)] (.*)$")
private val HEADING = Regex("^(#{1,6}) (.*)$")

fun parseMarkdown(md: String): List<Block> {
    val lines = md.split("\n")
    val out = mutableListOf<Block>()
    var i = 0
    val para = StringBuilder()
    fun flush() {
        if (para.isNotEmpty()) {
            out += Block.Para(para.toString())
            para.clear()
        }
    }
    while (i < lines.size) {
        val line = lines[i]
        val t = line.trimStart()
        when {
            t.startsWith("```") || t.startsWith("~~~") -> {
                flush()
                val code = StringBuilder()
                i++
                while (i < lines.size && !lines[i].trimStart().let { it.startsWith("```") || it.startsWith("~~~") }) {
                    if (code.isNotEmpty()) code.append('\n')
                    code.append(lines[i])
                    i++
                }
                out += Block.Code(code.toString())
            }
            t.isEmpty() -> {
                flush()
                if (out.lastOrNull() != Block.Space) out += Block.Space
            }
            HEADING.matches(t) -> {
                flush()
                val m = HEADING.find(t)!!
                out += Block.Heading(m.groupValues[1].length, m.groupValues[2])
            }
            TASK.matches(line) -> {
                flush()
                val m = TASK.find(line)!!
                out += Block.Task(m.groupValues[2] != " ", m.groupValues[3], i)
            }
            BULLET.matches(line) -> {
                flush()
                val m = BULLET.find(line)!!
                out += Block.Bullet(m.groupValues[2], m.groupValues[1].length / 2)
            }
            ORDERED.matches(line) -> {
                flush()
                val m = ORDERED.find(line)!!
                out += Block.Ordered(m.groupValues[1], m.groupValues[2])
            }
            t.startsWith(">") -> {
                flush()
                out += Block.Quote(t.removePrefix(">").trimStart())
            }
            t.length >= 3 && (t.all { it == '-' } || t.all { it == '*' } || t.all { it == '_' }) -> {
                flush()
                out += Block.Rule
            }
            t.startsWith("|") -> {
                flush()
                val rows = mutableListOf<List<String>>()
                while (i < lines.size && lines[i].trimStart().startsWith("|")) {
                    val cells = lines[i].trim().trim('|').split("|").map { it.trim() }
                    if (!cells.all { c -> c.isNotEmpty() && c.all { it == '-' || it == ':' } }) rows += cells
                    i++
                }
                i--
                out += Block.Table(rows)
            }
            else -> {
                if (para.isNotEmpty()) para.append(' ')
                para.append(t)
            }
        }
        i++
    }
    flush()
    return out
}

/** Отметить / снять галочку в строке задачи. */
fun toggleTask(text: String, line: Int): String {
    val lines = text.split("\n").toMutableList()
    if (line !in lines.indices) return text
    val l = lines[line]
    lines[line] = when {
        "[ ]" in l -> l.replaceFirst("[ ]", "[x]")
        "[x]" in l -> l.replaceFirst("[x]", "[ ]")
        "[X]" in l -> l.replaceFirst("[X]", "[ ]")
        else -> l
    }
    return lines.joinToString("\n")
}

// ------------------------------------------------------------------ строчная разметка

fun inlineMarkdown(s: String, cs: ColorScheme): AnnotatedString {
    val b = AnnotatedString.Builder()
    appendInline(b, s, cs)
    return b.toAnnotatedString()
}

private fun appendInline(b: AnnotatedString.Builder, s: String, cs: ColorScheme) {
    var i = 0
    while (i < s.length) {
        val rest = s.substring(i)
        // **жирный** / __жирный__
        if (rest.startsWith("**") || rest.startsWith("__")) {
            val mark = rest.substring(0, 2)
            val e = s.indexOf(mark, i + 2)
            if (e > i + 2) {
                b.pushStyle(SpanStyle(fontWeight = FontWeight.Bold))
                appendInline(b, s.substring(i + 2, e), cs)
                b.pop()
                i = e + 2
                continue
            }
        }
        // ~~зачёркнутый~~
        if (rest.startsWith("~~")) {
            val e = s.indexOf("~~", i + 2)
            if (e > i + 2) {
                b.pushStyle(SpanStyle(textDecoration = TextDecoration.LineThrough))
                appendInline(b, s.substring(i + 2, e), cs)
                b.pop()
                i = e + 2
                continue
            }
        }
        val c = s[i]
        // `код`
        if (c == '`') {
            val e = s.indexOf('`', i + 1)
            if (e > i + 1) {
                b.pushStyle(SpanStyle(fontFamily = FontFamily.Monospace, background = cs.surfaceContainerHighest, color = cs.tertiary))
                b.append(" " + s.substring(i + 1, e) + " ")
                b.pop()
                i = e + 1
                continue
            }
        }
        // *курсив* / _курсив_
        if ((c == '*' || c == '_') && i + 1 < s.length && s[i + 1] != ' ') {
            val e = s.indexOf(c, i + 1)
            if (e > i + 1) {
                b.pushStyle(SpanStyle(fontStyle = FontStyle.Italic))
                appendInline(b, s.substring(i + 1, e), cs)
                b.pop()
                i = e + 1
                continue
            }
        }
        // [текст](ссылка)
        if (c == '[') {
            val close = s.indexOf("](", i)
            val end = if (close > 0) s.indexOf(')', close) else -1
            if (close > i && end > close) {
                val url = s.substring(close + 2, end)
                b.pushLink(LinkAnnotation.Url(url, TextLinkStyles(SpanStyle(color = cs.primary, textDecoration = TextDecoration.Underline))))
                b.append(s.substring(i + 1, close))
                b.pop()
                i = end + 1
                continue
            }
        }
        b.append(c)
        i++
    }
}

// ------------------------------------------------------------------ предпросмотр

@Composable
fun MarkdownView(text: String, fontSize: Float, onToggleTask: (Int) -> Unit, modifier: Modifier = Modifier) {
    val cs = MaterialTheme.colorScheme
    val blocks = remember(text) { parseMarkdown(text) }
    val body = MaterialTheme.typography.bodyLarge.copy(fontSize = fontSize.sp, lineHeight = (fontSize * 1.45f).sp, color = cs.onSurface)
    Column(modifier, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        for (block in blocks) {
            when (block) {
                is Block.Heading -> {
                    val style = when (block.level) {
                        1 -> MaterialTheme.typography.headlineMedium
                        2 -> MaterialTheme.typography.headlineSmall
                        3 -> MaterialTheme.typography.titleLarge
                        else -> MaterialTheme.typography.titleMedium
                    }
                    Text(
                        inlineMarkdown(block.text, cs), style = style, fontWeight = FontWeight.Bold,
                        color = if (block.level == 1) cs.primary else cs.onSurface,
                        modifier = Modifier.padding(top = 8.dp, bottom = 2.dp),
                    )
                }
                is Block.Para -> Text(inlineMarkdown(block.text, cs), style = body)
                is Block.Bullet -> Row(Modifier.padding(start = (block.indent * 16).dp), verticalAlignment = Alignment.Top) {
                    Box(
                        Modifier.padding(top = (fontSize * 0.55f).dp, end = 10.dp, start = 4.dp)
                            .size(8.dp).clip(CookieShape(6, 0.15f)).background(cs.primary),
                    )
                    Text(inlineMarkdown(block.text, cs), style = body)
                }
                is Block.Ordered -> Row {
                    Text("${block.number}.", style = body, color = cs.primary, fontWeight = FontWeight.Bold, modifier = Modifier.padding(end = 8.dp))
                    Text(inlineMarkdown(block.text, cs), style = body)
                }
                is Block.Task -> Row(verticalAlignment = Alignment.CenterVertically) {
                    // Галочки можно отмечать прямо пальцем
                    Checkbox(
                        checked = block.checked,
                        onCheckedChange = { onToggleTask(block.line) },
                        colors = CheckboxDefaults.colors(checkedColor = cs.primary, checkmarkColor = cs.onPrimary, uncheckedColor = cs.outline),
                    )
                    Text(
                        inlineMarkdown(block.text, cs),
                        style = body.copy(textDecoration = if (block.checked) TextDecoration.LineThrough else null),
                        modifier = Modifier.alpha(if (block.checked) 0.6f else 1f),
                    )
                }
                is Block.Quote -> Row(Modifier.height(IntrinsicSize.Min).padding(vertical = 2.dp)) {
                    Box(Modifier.width(4.dp).fillMaxHeight().clip(RoundedCornerShape(2.dp)).background(cs.primary))
                    Text(
                        inlineMarkdown(block.text, cs), style = body.copy(fontStyle = FontStyle.Italic, color = cs.onSurfaceVariant),
                        modifier = Modifier.padding(start = 12.dp),
                    )
                }
                is Block.Code -> Surface(shape = RoundedCornerShape(16.dp), color = cs.surfaceContainerHighest, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        block.text,
                        style = TextStyle(fontFamily = FontFamily.Monospace, fontSize = (fontSize * 0.85f).sp, color = cs.onSurface),
                        modifier = Modifier.horizontalScroll(rememberScrollState()).padding(14.dp),
                    )
                }
                is Block.Table -> Column(
                    Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).border(1.dp, cs.outlineVariant, RoundedCornerShape(12.dp)),
                ) {
                    block.rows.forEachIndexed { r, row ->
                        Row(Modifier.fillMaxWidth().background(if (r == 0) cs.secondaryContainer else cs.surface)) {
                            row.forEach { cell ->
                                Text(
                                    inlineMarkdown(cell, cs), style = body.copy(fontWeight = if (r == 0) FontWeight.Bold else null),
                                    modifier = Modifier.weight(1f).padding(10.dp),
                                )
                            }
                        }
                        if (r < block.rows.size - 1) HorizontalDivider(color = cs.outlineVariant)
                    }
                }
                Block.Rule -> HorizontalDivider(Modifier.padding(vertical = 8.dp), color = cs.outlineVariant)
                Block.Space -> Spacer(Modifier.height(6.dp))
            }
        }
        Spacer(Modifier.height(80.dp))
    }
}

// ------------------------------------------------------------------ подсветка в редакторе

class MarkdownHighlight(private val cs: ColorScheme) : VisualTransformation {
    override fun filter(text: AnnotatedString): TransformedText =
        TransformedText(highlight(text.text), OffsetMapping.Identity)

    private fun highlight(src: String): AnnotatedString {
        val b = AnnotatedString.Builder(src)
        var pos = 0
        var fence = false
        for (line in src.split("\n")) {
            val start = pos
            val end = pos + line.length
            val t = line.trimStart()
            val indent = line.length - t.length
            when {
                t.startsWith("```") || t.startsWith("~~~") -> {
                    fence = !fence
                    b.addStyle(SpanStyle(color = cs.outline), start, end)
                }
                fence -> b.addStyle(SpanStyle(fontFamily = FontFamily.Monospace, color = cs.tertiary), start, end)
                HEADING.matches(t) -> {
                    val marks = t.takeWhile { it == '#' }.length
                    b.addStyle(SpanStyle(color = cs.outline, fontWeight = FontWeight.Bold), start, start + indent + marks)
                    b.addStyle(SpanStyle(color = cs.primary, fontWeight = FontWeight.Bold), start + indent + marks, end)
                }
                t.startsWith(">") -> b.addStyle(SpanStyle(color = cs.onSurfaceVariant, fontStyle = FontStyle.Italic), start, end)
                else -> {
                    val marker = TASK.find(line)?.let { line.indexOf(']') + 1 }
                        ?: BULLET.find(line)?.let { indent + 1 }
                        ?: ORDERED.find(line)?.let { line.indexOf(' ', indent) }
                    if (marker != null && marker > 0) {
                        b.addStyle(SpanStyle(color = cs.primary, fontWeight = FontWeight.Bold), start, start + marker)
                    }
                    inlineHighlight(b, line, start)
                }
            }
            pos = end + 1
        }
        return b.toAnnotatedString()
    }

    private fun inlineHighlight(b: AnnotatedString.Builder, line: String, offset: Int) {
        var i = 0
        while (i < line.length) {
            if (line.startsWith("**", i)) {
                val e = line.indexOf("**", i + 2)
                if (e > i + 2) {
                    b.addStyle(SpanStyle(color = cs.outline), offset + i, offset + i + 2)
                    b.addStyle(SpanStyle(fontWeight = FontWeight.Bold), offset + i + 2, offset + e)
                    b.addStyle(SpanStyle(color = cs.outline), offset + e, offset + e + 2)
                    i = e + 2
                    continue
                }
            }
            if (line[i] == '`') {
                val e = line.indexOf('`', i + 1)
                if (e > i) {
                    b.addStyle(SpanStyle(fontFamily = FontFamily.Monospace, color = cs.tertiary, background = cs.surfaceContainerHighest), offset + i, offset + e + 1)
                    i = e + 1
                    continue
                }
            }
            if (line[i] == '[') {
                val close = line.indexOf("](", i)
                val end = if (close > 0) line.indexOf(')', close) else -1
                if (close > i && end > close) {
                    b.addStyle(SpanStyle(color = cs.primary, textDecoration = TextDecoration.Underline), offset + i, offset + end + 1)
                    i = end + 1
                    continue
                }
            }
            i++
        }
    }
}

// ------------------------------------------------------------------ форматирование

sealed interface Fmt {
    data class Wrap(val pre: String, val post: String, val placeholder: String) : Fmt
    data class Line(val prefix: String) : Fmt
    data class Insert(val block: String) : Fmt
}

fun Fmt.applyTo(v: TextFieldValue): TextFieldValue {
    val t = v.text
    val a = v.selection.min
    val b = v.selection.max
    return when (this) {
        is Fmt.Wrap -> when {
            a == b -> TextFieldValue(
                t.substring(0, a) + pre + placeholder + post + t.substring(a),
                TextRange(a + pre.length, a + pre.length + placeholder.length),
            )
            t.substring(0, a).endsWith(pre) && t.substring(b).startsWith(post) -> TextFieldValue(
                t.substring(0, a - pre.length) + t.substring(a, b) + t.substring(b + post.length),
                TextRange(a - pre.length, b - pre.length),
            )
            else -> TextFieldValue(
                t.substring(0, a) + pre + t.substring(a, b) + post + t.substring(b),
                TextRange(a + pre.length, b + pre.length),
            )
        }
        is Fmt.Line -> {
            val ls = if (a == 0) 0 else t.lastIndexOf('\n', a - 1) + 1
            if (t.startsWith(prefix, ls)) {
                val len = prefix.length
                fun shift(x: Int) = if (x >= ls + len) x - len else ls
                TextFieldValue(t.removeRange(ls, ls + len), TextRange(shift(a), shift(b)))
            } else {
                TextFieldValue(t.substring(0, ls) + prefix + t.substring(ls), TextRange(a + prefix.length, b + prefix.length))
            }
        }
        is Fmt.Insert -> TextFieldValue(t.substring(0, b) + block + t.substring(b), TextRange(b + block.length))
    }
}

val TOOLS: List<Pair<String, Fmt>> = listOf(
    "H1" to Fmt.Line("# "),
    "H2" to Fmt.Line("## "),
    "B" to Fmt.Wrap("**", "**", "жирный"),
    "I" to Fmt.Wrap("*", "*", "курсив"),
    "S̶" to Fmt.Wrap("~~", "~~", "текст"),
    "☑" to Fmt.Line("- [ ] "),
    "•" to Fmt.Line("- "),
    "1." to Fmt.Line("1. "),
    "❝" to Fmt.Line("> "),
    "</>" to Fmt.Wrap("`", "`", "код"),
    "🔗" to Fmt.Wrap("[", "](https://)", "ссылка"),
    "{ }" to Fmt.Wrap("```\n", "\n```", "код"),
    "⊞" to Fmt.Insert("\n| Столбец 1 | Столбец 2 |\n|---|---|\n| Ячейка | Ячейка |\n"),
    "―" to Fmt.Insert("\n---\n"),
)

/** Короткий текст для карточки в списке: без разметки. */
fun previewOf(note: Note): String =
    note.text.lines()
        .map {
            it.trim().trimStart('#', '>', ' ').replace("- [x]", "✓").replace("- [X]", "✓").replace("- [ ]", "○")
                .replace("**", "").replace("__", "").replace("`", "").replace("~~", "")
        }
        .filter { it.isNotBlank() && it.trimStart('-', ' ') != note.title }
        .joinToString(" · ")
        .take(180)
