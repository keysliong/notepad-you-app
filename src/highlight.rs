//! Лёгкая подсветка синтаксиса Markdown прямо в редакторе.

use crate::theme::{bold_family, Palette};
use eframe::egui::{text::LayoutJob, FontFamily, FontId, Stroke, TextFormat};

struct Formats {
    base: TextFormat,
    bold: TextFormat,
    italic: TextFormat,
    code: TextFormat,
    link: TextFormat,
}

pub fn layout_job(text: &str, p: &Palette, size: f32, mono: bool) -> LayoutJob {
    let body = if mono { FontFamily::Monospace } else { FontFamily::Proportional };
    let strong = if mono { FontFamily::Monospace } else { bold_family() };

    let base = TextFormat {
        font_id: FontId::new(size, body),
        color: p.on_surface,
        ..Default::default()
    };
    let f = Formats {
        bold: TextFormat { font_id: FontId::new(size, strong.clone()), ..base.clone() },
        italic: TextFormat { italics: true, ..base.clone() },
        code: TextFormat {
            font_id: FontId::new(size * 0.92, FontFamily::Monospace),
            color: p.tertiary,
            background: p.surface_container_highest,
            ..base.clone()
        },
        link: TextFormat { color: p.primary, underline: Stroke::new(1.0_f32, p.primary), ..base.clone() },
        base,
    };
    let muted = TextFormat { color: p.outline, ..f.base.clone() };
    let code_block = TextFormat {
        font_id: FontId::new(size * 0.92, FontFamily::Monospace),
        color: p.tertiary,
        ..f.base.clone()
    };

    let mut job = LayoutJob::default();
    if text.is_empty() {
        job.append("", 0.0, f.base.clone());
        return job;
    }

    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Блоки кода ``` … ```
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            job.append(line, 0.0, muted.clone());
            continue;
        }
        if in_fence {
            job.append(line, 0.0, code_block.clone());
            continue;
        }

        // Заголовки
        let hashes = trimmed.bytes().take_while(|b| *b == b'#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            let sz = size + (7 - hashes) as f32 * 1.6;
            let head = TextFormat {
                font_id: FontId::new(sz, strong.clone()),
                color: p.primary,
                ..f.base.clone()
            };
            let marks = TextFormat { font_id: FontId::new(sz, strong.clone()), color: p.outline, ..f.base.clone() };
            job.append(&line[..indent + hashes], 0.0, marks);
            job.append(&line[indent + hashes..], 0.0, head);
            continue;
        }

        // Цитаты
        if trimmed.starts_with('>') {
            let quote = TextFormat { color: p.on_surface_variant, italics: true, ..f.base.clone() };
            job.append(line, 0.0, quote);
            continue;
        }

        // Горизонтальная линия
        let t = trimmed.trim_end();
        if t.len() >= 3
            && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*') || t.chars().all(|c| c == '_'))
        {
            job.append(line, 0.0, muted.clone());
            continue;
        }

        // Списки
        let marker = list_marker_len(trimmed);
        if marker > 0 {
            let m = TextFormat { color: p.primary, ..f.bold.clone() };
            job.append(&line[..indent + marker], 0.0, m);
            inline(&mut job, &line[indent + marker..], &f);
            continue;
        }

        inline(&mut job, line, &f);
    }
    hide_emoji(job)
}

/// Смайлики в тексте делаем прозрачными: поверх них редактор рисует цветные картинки.
/// Первый символ оставляет место под картинку, остальные части последовательности
/// (ZWJ, селекторы, оттенки кожи) делаем почти нулевой ширины.
fn hide_emoji(job: LayoutJob) -> LayoutJob {
    use eframe::egui::{text::LayoutSection, Color32};
    let found = crate::emoji::clusters(&job.text);
    if found.is_empty() {
        return job;
    }
    let mut out = job.clone();
    out.sections.clear();
    for sec in &job.sections {
        let (s, e) = (sec.byte_range.start, sec.byte_range.end);
        let mut pos = s;
        let push = |out: &mut LayoutJob, range: std::ops::Range<usize>, format: TextFormat| {
            if range.start < range.end {
                let leading_space = if range.start == s { sec.leading_space } else { 0.0 };
                out.sections.push(LayoutSection { leading_space, byte_range: range, format });
            }
        };
        for &(a, b) in found.iter().filter(|(a, b)| *a < e && *b > s) {
            let (a, b) = (a.max(s), b.min(e));
            push(&mut out, pos..a, sec.format.clone());
            let first_end = (a + job.text[a..].chars().next().map_or(1, char::len_utf8)).min(b);
            let mut hidden = sec.format.clone();
            hidden.color = Color32::TRANSPARENT;
            hidden.background = Color32::TRANSPARENT;
            push(&mut out, a..first_end, hidden.clone());
            hidden.font_id.size = 1.0;
            push(&mut out, first_end..b, hidden);
            pos = b;
        }
        push(&mut out, pos..e, sec.format.clone());
    }
    out
}

fn list_marker_len(s: &str) -> usize {
    for task in ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] "] {
        if s.starts_with(task) {
            return task.len();
        }
    }
    let b = s.as_bytes();
    if b.len() >= 2 && matches!(b[0], b'-' | b'*' | b'+') && b[1] == b' ' {
        return 2;
    }
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && b.len() >= digits + 2 && matches!(b[digits], b'.' | b')') && b[digits + 1] == b' ' {
        return digits + 2;
    }
    0
}

/// Внутристрочная разметка: `код`, **жирный**, *курсив*, [ссылка](url).
fn inline(job: &mut LayoutJob, s: &str, f: &Formats) {
    let b = s.as_bytes();
    let mut i = 0;
    let mut start = 0;
    let flush = |job: &mut LayoutJob, from: usize, to: usize| {
        if to > from {
            job.append(&s[from..to], 0.0, f.base.clone());
        }
    };
    while i < b.len() {
        let mut span: Option<(usize, &TextFormat)> = None;
        match b[i] {
            b'`' => {
                if let Some(e) = s[i + 1..].find('`') {
                    span = Some((i + 1 + e + 1, &f.code));
                }
            }
            b'*' | b'_' if i + 1 < b.len() && b[i + 1] == b[i] => {
                let pat = if b[i] == b'*' { "**" } else { "__" };
                if let Some(e) = s[i + 2..].find(pat) {
                    if e > 0 {
                        span = Some((i + 2 + e + 2, &f.bold));
                    }
                }
            }
            b'*' if i + 1 < b.len() && b[i + 1] != b' ' => {
                if let Some(e) = s[i + 1..].find('*') {
                    if e > 0 {
                        span = Some((i + 1 + e + 1, &f.italic));
                    }
                }
            }
            b'[' => {
                if let Some(e) = s[i..].find("](") {
                    if let Some(e2) = s[i + e..].find(')') {
                        span = Some((i + e + e2 + 1, &f.link));
                    }
                }
            }
            _ => {}
        }
        if let Some((end, fmt)) = span {
            flush(job, start, i);
            job.append(&s[i..end], 0.0, fmt.clone());
            i = end;
            start = end;
        } else {
            i += 1;
        }
    }
    flush(job, start, b.len());
}
