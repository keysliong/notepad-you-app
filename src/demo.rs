//! Скрытый демо-режим: проигрывает сценарий и записывает кадры — для гифок в README.
//!
//! Включается только переменными окружения, обычный запуск его не затрагивает:
//!   NPY_DEMO=themes NPY_RECORD=папка [NPY_SCALE=2] NotepadYou.exe
//! Сценарии: hero, dark, themes, typing, tutorial, notes.
//! В демо-режиме настройки и сессия не читаются и не сохраняются.

use crate::app::ViewMode;
use crate::theme::Seed;
use eframe::egui::ColorImage;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

pub enum Act {
    Seed(Seed),
    Dark(bool),
    Settings(bool),
    NewDoc,
    Type(&'static str),
    View(ViewMode),
    Select(usize),
    Close(usize),
    TutStart,
    TutNext,
    /// Снять один кадр (для статичных скриншотов)
    Shot,
    End,
}

pub const NOTE: &str = "# Планы на выходные 🌿

Список дел на **субботу** и *воскресенье*:

- [x] Купить продукты 🛒
- [x] Позвонить бабушке 📞
- [ ] Испечь печеньки 🍪
- [ ] Сходить в кино 🎬

> Не забыть зонтик ☔

## Идеи

1. Поехать за город 🚗
2. Сыграть в настолки 🎲

```rust
fn main() {
    println!(\"Привет, Notepad You!\");
}
```
";

pub const NOTE2: &str = "# Идеи для проекта 💡\n\n- Тёмная тема 🌙\n- Анимации ✨\n- Кнопки-печеньки 🍪\n";
pub const NOTE3: &str = "# Конспект 📚\n\nКороткие заметки с лекции.\n";

const TYPED: &str = "# Мой список 📝\n\nСегодня **важно**:\n\n- [x] Проснуться ☀️\n- [ ] Выпить чай 🍵\n- [ ] Написать заметку ✍️\n\n> Всё получится! 💜\n";

pub struct Demo {
    timeline: Vec<(f32, Act)>,
    next: usize,
    warmup: u32,
    start: Option<Instant>,
    out: Option<PathBuf>,
    scale: usize,
    record_all: bool,
    want_shot: bool,
    frames: usize,
    log: String,
    typing: Option<(&'static str, f32, usize)>,
}

impl Demo {
    pub fn from_env() -> Option<Self> {
        let script = std::env::var("NPY_DEMO").ok()?;
        let out = std::env::var("NPY_RECORD").ok().map(PathBuf::from);
        if let Some(o) = &out {
            let _ = std::fs::create_dir_all(o);
        }
        let scale = std::env::var("NPY_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(2).max(1);
        use Act::*;
        let timeline: Vec<(f32, Act)> = match script.as_str() {
            "hero" => vec![(1.6, Shot), (2.0, End)],
            "dark" => vec![(0.0, Dark(true)), (0.0, Seed(crate::theme::Seed::Lavender)), (1.8, Shot), (2.2, End)],
            "themes" => {
                let mut t = vec![(0.6, Settings(true))];
                let seeds = [
                    crate::theme::Seed::Coral,
                    crate::theme::Seed::Ocean,
                    crate::theme::Seed::Mint,
                    crate::theme::Seed::Rose,
                    crate::theme::Seed::Amber,
                ];
                let mut at = 1.4;
                for s in seeds {
                    t.push((at, Seed(s)));
                    at += 1.0;
                }
                t.push((at, Dark(true)));
                at += 1.1;
                for s in [crate::theme::Seed::Graphite, crate::theme::Seed::Ocean, crate::theme::Seed::Lavender] {
                    t.push((at, Seed(s)));
                    at += 1.0;
                }
                t.push((at, Settings(false)));
                t.push((at + 1.0, End));
                t
            }
            "typing" => vec![(0.5, NewDoc), (1.2, Type(TYPED)), (10.5, End)],
            "tutorial" => vec![
                (0.2, TutStart),
                (2.4, TutNext),
                (4.0, NewDoc),
                (5.6, Type("Привет! 👋")),
                (8.0, TutNext),
                (9.6, View(ViewMode::Preview)),
                (11.2, TutNext),
                (12.8, Settings(true)),
                (14.0, Seed(crate::theme::Seed::Mint)),
                (15.4, TutNext),
                (17.0, TutNext),
                (18.8, TutNext),
                (19.8, End),
            ],
            "notes" => vec![
                (0.6, NewDoc),
                (1.4, NewDoc),
                (2.3, Select(0)),
                (3.1, Select(2)),
                (4.0, Close(3)),
                (4.9, View(ViewMode::Edit)),
                (5.8, View(ViewMode::Preview)),
                (6.7, View(ViewMode::Split)),
                (7.6, End),
            ],
            _ => vec![(3.0, End)],
        };
        let record_all = !matches!(script.as_str(), "hero" | "dark");
        Some(Self {
            timeline,
            next: 0,
            warmup: 0,
            start: None,
            out,
            scale,
            record_all,
            want_shot: false,
            frames: 0,
            log: String::new(),
            typing: None,
        })
    }

    /// Время с начала сценария (None — ещё «прогрев»: окно открывается).
    pub fn elapsed(&mut self) -> Option<f32> {
        if self.start.is_none() {
            self.warmup += 1;
            if self.warmup < 40 {
                return None;
            }
            self.start = Some(Instant::now());
        }
        self.start.map(|s| s.elapsed().as_secs_f32())
    }

    /// Действия, время которых наступило.
    pub fn due(&mut self, t: f32) -> Vec<Act> {
        let mut out = Vec::new();
        while self.next < self.timeline.len() && self.timeline[self.next].0 <= t {
            let act = std::mem::replace(&mut self.timeline[self.next].1, Act::Shot);
            if matches!(act, Act::Shot) {
                self.want_shot = true;
            }
            out.push(act);
            self.next += 1;
        }
        out
    }

    pub fn start_typing(&mut self, text: &'static str, t: f32) {
        self.typing = Some((text, t, 0));
    }

    /// Сколько новых символов «напечатать» к моменту t (≈ 17 символов в секунду).
    pub fn typed_chars(&mut self, t: f32) -> Option<String> {
        let (text, t0, done) = self.typing.as_mut()?;
        let total = text.chars().count();
        let due = (((t - *t0) * 17.0) as usize).min(total);
        if due <= *done {
            return None;
        }
        let s: String = text.chars().skip(*done).take(due - *done).collect();
        *done = due;
        if due >= total {
            self.typing = None;
        }
        Some(s)
    }

    pub fn wants_screenshot(&mut self) -> bool {
        let want = self.out.is_some() && self.start.is_some() && (self.record_all || self.want_shot);
        self.want_shot = false;
        want
    }

    /// Сохраняет кадр: уменьшенный в `scale` раз RGBA с заголовком (ширина, высота).
    pub fn save_frame(&mut self, img: &ColorImage) {
        let Some(dir) = &self.out else { return };
        let Some(start) = self.start else { return };
        let k = self.scale;
        let (w, h) = (img.size[0] / k, img.size[1] / k);
        let mut buf = Vec::with_capacity(8 + w * h * 4);
        buf.extend_from_slice(&(w as u32).to_le_bytes());
        buf.extend_from_slice(&(h as u32).to_le_bytes());
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 4];
                for dy in 0..k {
                    for dx in 0..k {
                        let c = img.pixels[(y * k + dy) * img.size[0] + x * k + dx];
                        acc[0] += c.r() as u32;
                        acc[1] += c.g() as u32;
                        acc[2] += c.b() as u32;
                        acc[3] += c.a() as u32;
                    }
                }
                let n = (k * k) as u32;
                buf.extend_from_slice(&[(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8, 255]);
            }
        }
        let path = dir.join(format!("frame_{:05}.rgba", self.frames));
        if let Ok(mut f) = std::fs::File::create(path) {
            let _ = f.write_all(&buf);
        }
        let _ = writeln!(self.log, "{} {}", self.frames, start.elapsed().as_millis());
        self.frames += 1;
    }

    pub fn finish(&self) {
        if let Some(dir) = &self.out {
            let _ = std::fs::write(dir.join("frames.txt"), &self.log);
        }
    }
}
