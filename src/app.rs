//! Основное приложение Notepad You.

use crate::highlight;
use crate::theme::{self, bold_family, mix, Palette, Seed, UiFont};
use crate::widgets::{self, cookie, ease_out, icon_button, icon_font, pill, segmented, switch, tool_button, Icons};
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange, LayoutJob, TextWrapping},
    vec2, Align, Align2, CentralPanel, Color32, Frame, Id, Key, KeyboardShortcut, LayerId, Layout,
    Margin, Modifiers, Order, Pos2, Rect, RichText, ScrollArea, Sense, SidePanel, Stroke, TextEdit,
    TextStyle, TopBottomPanel, Ui, ViewportCommand,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

const WELCOME: &str = r#"# Добро пожаловать в Notepad You 👋

Красивый блокнот в стиле **Material You** с поддержкой *Markdown*. Смайлики тоже цветные 🍪✨🎨

## Что умеет
- Редактирование и **предпросмотр** `.md` файлов
- Подсветка разметки прямо в редакторе
- Несколько заметок — список слева
- Цветовые темы и тёмный режим (кнопка настроек справа вверху)
- Перетаскивание файлов прямо в окно

## Горячие клавиши
| Действие | Клавиши |
|---|---|
| Новая заметка | Ctrl+N |
| Открыть | Ctrl+O |
| Сохранить | Ctrl+S |
| Сохранить как | Ctrl+Shift+S |
| Закрыть заметку | Ctrl+W |
| Жирный / курсив | Ctrl+B / Ctrl+I |
| Ссылка | Ctrl+K |
| Режим просмотра | Ctrl+E |
| Следующая заметка | Ctrl+Tab |

> Совет: режим «Обе» показывает редактор и предпросмотр рядом.

- [x] Установить Notepad You 🎉
- [ ] Написать первую заметку ✍️

```rust
fn main() {
    println!("Привет, мир!");
}
```
"#;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Edit,
    Split,
    Preview,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub seed: Seed,
    pub dark: bool,
    pub font_size: f32,
    pub mono: bool,
    pub view: ViewMode,
    pub sidebar: bool,
    pub session: Vec<PathBuf>,
    pub active: usize,
    pub font: UiFont,
    pub tutorial_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tutorial_done: false,
            font: UiFont::Nunito,
            seed: Seed::Lavender,
            dark: false,
            font_size: 16.0,
            mono: false,
            view: ViewMode::Split,
            sidebar: true,
            session: Vec::new(),
            active: 0,
        }
    }
}

struct Doc {
    id: u64,
    path: Option<PathBuf>,
    name: String,
    text: String,
    dirty: bool,
    crlf: bool,
    cache: CommonMarkCache,
    /// Когда заметка появилась (анимация появления в списке)
    born: Instant,
    /// Когда начали закрывать (анимация исчезновения)
    closing: Option<Instant>,
}

impl Doc {
    fn new(id: u64, path: Option<PathBuf>, name: String, text: String, crlf: bool) -> Self {
        Self {
            id,
            path,
            name,
            text,
            dirty: false,
            crlf,
            cache: CommonMarkCache::default(),
            born: Instant::now(),
            closing: None,
        }
    }

    fn title(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.name.clone())
    }
}

#[derive(Clone, Copy)]
enum Pending {
    /// id заметки
    CloseDoc(u64),
    Exit,
}

/// Команда форматирования Markdown.
#[derive(Clone, Copy)]
enum Fmt {
    /// Обернуть выделение: префикс, суффикс, текст-заглушка
    Wrap(&'static str, &'static str, &'static str),
    /// Префикс в начале строки (переключается)
    Line(&'static str),
    /// Вставить блок текста
    Insert(&'static str),
}

fn byte_at(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map(|(i, _)| i).unwrap_or(s.len())
}

impl Fmt {
    /// Применяет форматирование к выделению [a, b) (в символах), возвращает новое выделение.
    fn apply(self, text: &mut String, a: usize, b: usize) -> (usize, usize) {
        match self {
            Fmt::Wrap(pre, post, ph) => {
                let ia = byte_at(text, a);
                let ib = byte_at(text, b);
                let pl = pre.chars().count();
                if a == b {
                    text.insert_str(ia, &format!("{pre}{ph}{post}"));
                    (a + pl, a + pl + ph.chars().count())
                } else if text[..ia].ends_with(pre) && text[ib..].starts_with(post) {
                    text.replace_range(ib..ib + post.len(), "");
                    text.replace_range(ia - pre.len()..ia, "");
                    (a - pl, b - pl)
                } else {
                    text.insert_str(ib, post);
                    text.insert_str(ia, pre);
                    (a + pl, b + pl)
                }
            }
            Fmt::Line(prefix) => {
                let ia = byte_at(text, a);
                let ls = text[..ia].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let ls_char = text[..ls].chars().count();
                let pl = prefix.chars().count();
                if text[ls..].starts_with(prefix) {
                    text.replace_range(ls..ls + prefix.len(), "");
                    let shift = |x: usize| if x >= ls_char + pl { x - pl } else { ls_char.min(x) };
                    (shift(a), shift(b))
                } else {
                    text.insert_str(ls, prefix);
                    (a + pl, b + pl)
                }
            }
            Fmt::Insert(block) => {
                let ib = byte_at(text, b);
                text.insert_str(ib, block);
                let end = b + block.chars().count();
                (end, end)
            }
        }
    }
}

// ------------------------------------------------------------------ обучение

/// Чего ждём от пользователя, чтобы перейти к следующему шагу.
#[derive(Clone, Copy, PartialEq)]
enum Wait {
    /// Кнопка «Далее»
    Next,
    /// Создать заметку
    NewDoc,
    /// Напечатать текст
    Type,
    /// Сменить режим просмотра
    View,
    /// Открыть настройки
    Settings,
}

struct Step {
    target: Option<&'static str>,
    title: &'static str,
    text: &'static str,
    wait: Wait,
}

const STEPS: &[Step] = &[
    Step {
        target: None,
        title: "Добро пожаловать в Notepad You!",
        text: "Давайте я быстро покажу, как здесь всё устроено. Нужные места будут подсвечиваться, а я буду подсказывать, что нажать.",
        wait: Wait::Next,
    },
    Step {
        target: Some("fab"),
        title: "Создайте заметку",
        text: "Нажмите на подсвеченную кнопку «Новая заметка».",
        wait: Wait::NewDoc,
    },
    Step {
        target: Some("editor"),
        title: "Напишите что-нибудь",
        text: "Кликните в подсвеченную область и напечатайте пару слов. Работает Markdown: # заголовок, **жирный**, - список.",
        wait: Wait::Type,
    },
    Step {
        target: Some("toolbar"),
        title: "Панель форматирования",
        text: "Выделите текст и нажмите кнопку — например, B сделает его жирным. Наведите курсор на кнопку, чтобы увидеть подсказку.",
        wait: Wait::Next,
    },
    Step {
        target: Some("view"),
        title: "Режимы просмотра",
        text: "Нажмите «Редактор» или «Просмотр», чтобы переключить режим. «Обе» — редактор и предпросмотр рядом.",
        wait: Wait::View,
    },
    Step {
        target: Some("save"),
        title: "Сохранение",
        text: "Эта кнопка сохраняет заметку в файл .md. Ещё быстрее — Ctrl+S.",
        wait: Wait::Next,
    },
    Step {
        target: Some("settings"),
        title: "Настройки",
        text: "Нажмите на шестерёнку, чтобы открыть настройки.",
        wait: Wait::Settings,
    },
    Step {
        target: Some("settings_panel"),
        title: "Сделайте по-своему",
        text: "Здесь цвет оформления, тёмная тема, шрифт и размер текста. Попробуйте нажать на любую цветную печеньку!",
        wait: Wait::Next,
    },
    Step {
        target: Some("notes"),
        title: "Ваши заметки",
        text: "Все открытые заметки — в этом списке. Наведите на заметку, чтобы закрыть её крестиком.",
        wait: Wait::Next,
    },
    Step {
        target: None,
        title: "Готово!",
        text: "Теперь вы знаете всё главное. Пройти обучение ещё раз можно в настройках.",
        wait: Wait::Next,
    },
];

struct Tutorial {
    step: usize,
    started: Instant,
    step_since: Instant,
    closing: Option<Instant>,
    hole: Option<Rect>,
    card_pos: Option<Pos2>,
    card_h: f32,
    view0: ViewMode,
    doc0: u64,
    len0: usize,
}

/// Логотип для боковой панели в цветах темы.
fn make_logo(ctx: &egui::Context, seed: Seed) -> egui::TextureHandle {
    let colors = theme::icon_colors(seed);
    ctx.load_texture(
        "logo",
        egui::ColorImage::from_rgba_unmultiplied([128, 128], &crate::icon::icon_rgba_with(128, &colors)),
        egui::TextureOptions::LINEAR,
    )
}

/// На macOS значок в Dock берётся из .app (AppIcon.icns), менять нечего.
#[cfg(not(windows))]
fn set_window_icon(_frame: &eframe::Frame, _seed: Seed) -> bool {
    true
}

/// Значок окна (заголовок и панель задач) в цветах темы — через WM_SETICON,
/// сразу большой и маленький размеры. Иконка самого .exe в Проводнике остаётся фиолетовой.
#[cfg(windows)]
fn set_window_icon(frame: &eframe::Frame, seed: Seed) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = frame.window_handle() else { return false };
    let RawWindowHandle::Win32(w) = handle.as_raw() else { return false };
    let hwnd = w.hwnd.get();
    let colors = theme::icon_colors(seed);
    unsafe {
        for (kind, metric) in [(1usize, 11), (0usize, 49)] {
            // ICON_BIG = 1 / SM_CXICON = 11, ICON_SMALL = 0 / SM_CXSMICON = 49
            let size = win::GetSystemMetrics(metric).clamp(16, 256) as u32;
            let dib = icon_dib(&crate::icon::icon_rgba_with(size, &colors), size);
            let hicon = win::CreateIconFromResourceEx(dib.as_ptr(), dib.len() as u32, 1, 0x0003_0000, size as i32, size as i32, 0);
            if hicon != 0 {
                win::SendMessageW(hwnd, 0x0080, kind, hicon); // WM_SETICON
            }
        }
    }
    true
}

/// RGBA → данные иконки в формате DIB (как внутри .ico).
#[cfg(windows)]
fn icon_dib(rgba: &[u8], n: u32) -> Vec<u8> {
    let mut b = Vec::with_capacity(40 + rgba.len() + (n * n / 8) as usize + 64);
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(n as i32).to_le_bytes());
    b.extend_from_slice(&((n * 2) as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    for y in (0..n).rev() {
        for x in 0..n {
            let i = ((y * n + x) * 4) as usize;
            b.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    let mask_row = n.div_ceil(32) * 4;
    b.extend(std::iter::repeat_n(0u8, (mask_row * n) as usize));
    b
}

#[cfg(windows)]
mod win {
    #[link(name = "user32")]
    extern "system" {
        pub fn GetSystemMetrics(index: i32) -> i32;
        pub fn CreateIconFromResourceEx(
            bits: *const u8,
            size: u32,
            icon: i32,
            version: u32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> isize;
        pub fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    }
}

fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect::from_min_max(a.min.lerp(b.min, t), a.max.lerp(b.max, t))
}

pub struct NotepadApp {
    settings: Settings,
    /// Цветные смайлики и галочки (см. emoji.rs)
    emoji: std::sync::Arc<crate::emoji::Emoji>,
    logo: egui::TextureHandle,
    /// Для какой темы нарисован логотип
    logo_seed: Seed,
    /// Предыдущий логотип — плавно растворяется при смене темы
    logo_prev: Option<(egui::TextureHandle, Instant)>,
    /// Значок окна можно сменить только когда окно уже создано — на первом кадре
    window_icon_sent: bool,
    targets: HashMap<&'static str, Rect>,
    tutorial: Option<Tutorial>,
    ev_new_doc: bool,
    docs: Vec<Doc>,
    active: usize,
    next_id: u64,
    untitled: u32,
    pending: Option<Pending>,
    force_exit: bool,
    fmt_request: Option<Fmt>,
    snackbar: Option<(String, Instant)>,
    applied: Option<(Seed, bool)>,
    last_title: String,
    icons: Icons,
    // анимации
    settings_open: bool,
    fonts_applied: UiFont,
    pal_from: Palette,
    pal_cur: Palette,
    pal_since: Option<Instant>,
    content_key: (u64, ViewMode),
    content_since: Instant,
    dialog_text: (String, String),
}

impl NotepadApp {
    pub fn new(cc: &eframe::CreationContext<'_>, file: Option<PathBuf>) -> Self {
        let settings: Settings = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        let has_icons = theme::setup_fonts(&cc.egui_ctx, settings.font);
        let pal = Palette::new(settings.seed, settings.dark);

        let logo = make_logo(&cc.egui_ctx, settings.seed);
        let emoji = crate::emoji::Emoji::new();
        cc.egui_ctx.add_texture_loader(std::sync::Arc::new(crate::emoji::Loader(emoji.clone())));
        let mut app = Self {
            emoji,
            window_icon_sent: false,
            logo,
            logo_seed: settings.seed,
            logo_prev: None,
            targets: HashMap::new(),
            tutorial: None,
            ev_new_doc: false,
            settings_open: false,
            fonts_applied: settings.font,
            pal_from: pal,
            pal_cur: pal,
            pal_since: None,
            content_key: (0, ViewMode::Split),
            content_since: Instant::now(),
            dialog_text: (String::new(), String::new()),
            settings,
            docs: Vec::new(),
            active: 0,
            next_id: 1,
            untitled: 0,
            pending: None,
            force_exit: false,
            fmt_request: None,
            snackbar: None,
            applied: None,
            last_title: String::new(),
            icons: if has_icons { widgets::MDL2 } else { widgets::FALLBACK },
        };

        // Восстанавливаем прошлую сессию
        let session = std::mem::take(&mut app.settings.session);
        for p in &session {
            if p.is_file() {
                app.open_path(p);
            }
        }
        app.active = app.settings.active.min(app.docs.len().saturating_sub(1));

        if let Some(f) = file {
            app.open_path(&f);
        }
        if app.docs.is_empty() {
            let id = app.alloc_id();
            app.docs.push(Doc::new(id, None, "Добро пожаловать".into(), WELCOME.into(), false));
            app.active = 0;
        }
        if !app.settings.tutorial_done {
            app.start_tutorial();
        }
        app
    }

    // ------------------------------------------------------------ обучение

    fn start_tutorial(&mut self) {
        self.settings.sidebar = true;
        if self.settings.view == ViewMode::Preview {
            self.settings.view = ViewMode::Split;
        }
        self.settings_open = false;
        let now = Instant::now();
        let mut tut = Tutorial {
            step: 0,
            started: now,
            step_since: now,
            closing: None,
            hole: None,
            card_pos: None,
            card_h: 220.0,
            view0: self.settings.view,
            doc0: 0,
            len0: 0,
        };
        self.tut_enter(&mut tut, 0);
        self.tutorial = Some(tut);
    }

    fn tut_enter(&mut self, tut: &mut Tutorial, i: usize) {
        tut.step = i;
        tut.step_since = Instant::now();
        tut.view0 = self.settings.view;
        let d = &self.docs[self.active];
        tut.doc0 = d.id;
        tut.len0 = d.text.chars().count();
        self.ev_new_doc = false;
    }

    fn tut_advance(&mut self, tut: &mut Tutorial) {
        if tut.step + 1 >= STEPS.len() {
            self.tut_finish(tut);
        } else {
            self.tut_enter(tut, tut.step + 1);
        }
    }

    fn tut_finish(&mut self, tut: &mut Tutorial) {
        tut.closing = Some(Instant::now());
        self.settings.tutorial_done = true;
        self.settings_open = false;
    }

    /// Затемнение с «окошком» над нужным элементом, пульсирующая рамка и карточка-подсказка.
    fn tutorial_ui(&mut self, ctx: &egui::Context, p: &Palette) {
        if self.pending.is_some() {
            return;
        }
        let Some(mut tut) = self.tutorial.take() else { return };
        let screen = ctx.screen_rect();
        let time = ctx.input(|i| i.time) as f32;
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        ctx.request_repaint();

        let appear = ease_out(tut.started.elapsed().as_secs_f32() / 0.45);
        let vanish = tut.closing.map(|t| ease_out(t.elapsed().as_secs_f32() / 0.3)).unwrap_or(0.0);
        if vanish >= 1.0 {
            return; // обучение закончено
        }
        let a = appear * (1.0 - vanish);
        let step = &STEPS[tut.step];

        // Окошко плавно «перелетает» к новому элементу
        let target = step.target.and_then(|k| self.targets.get(k).copied()).map(|r| r.expand(8.0));
        let k = 1.0 - (-dt * 9.0).exp();
        tut.hole = match (tut.hole, target) {
            (None, Some(t)) => Some(lerp_rect(screen, t, k)),
            (Some(h), Some(t)) => Some(lerp_rect(h, t, k)),
            (Some(h), None) => {
                let r = lerp_rect(h, Rect::from_center_size(h.center(), vec2(0.0, 0.0)), k);
                (r.width() > 1.0).then_some(r)
            }
            (None, None) => None,
        };

        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("tut-scrim")));
        let scrim = Color32::from_black_alpha((165.0 * a) as u8);
        if let Some(h) = tut.hole {
            let r = 18.0_f32.min(h.width().min(h.height()) / 2.0);
            // Очень толстая обводка = затемнение всего, кроме окошка со скруглёнными углами
            let w = 8000.0_f32;
            painter.rect_stroke(h.expand(w / 2.0), r + w / 2.0, Stroke::new(w, scrim));
            let pulse = 0.5 + 0.5 * (time * 3.2).sin();
            let e = 2.0 + 3.0 * pulse;
            painter.rect_stroke(h.expand(e), r + e, Stroke::new(2.5_f32, p.primary.gamma_multiply(a * (0.55 + 0.45 * pulse))));
            if step.wait != Wait::Next {
                // «Волны» — сюда нужно нажать
                for i in 0..2 {
                    let ph = (time / 1.4 + i as f32 * 0.5).fract();
                    let e = 4.0 + ph * 24.0;
                    painter.rect_stroke(h.expand(e), r + e, Stroke::new(2.0_f32, p.primary.gamma_multiply(a * (1.0 - ph) * 0.7)));
                }
            }
        } else {
            painter.rect_filled(screen, 0.0, scrim);
        }

        // Положение карточки: под элементом, над ним, сбоку или внутри
        let card_w = 360.0;
        let ch = tut.card_h;
        let desired = match (tut.hole, target) {
            (Some(h), Some(_)) => {
                let cx = (h.center().x - card_w / 2.0).clamp(16.0, screen.right() - card_w - 16.0);
                if h.bottom() + 18.0 + ch < screen.bottom() - 16.0 {
                    egui::pos2(cx, h.bottom() + 18.0)
                } else if h.top() - 18.0 - ch > screen.top() + 16.0 {
                    egui::pos2(cx, h.top() - 18.0 - ch)
                } else {
                    let cy = (h.center().y - ch / 2.0).clamp(16.0, screen.bottom() - ch - 16.0);
                    if h.right() + 18.0 + card_w < screen.right() - 16.0 {
                        egui::pos2(h.right() + 18.0, cy)
                    } else if h.left() - 18.0 - card_w > screen.left() + 16.0 {
                        egui::pos2(h.left() - 18.0 - card_w, cy)
                    } else {
                        egui::pos2(h.right() - card_w - 24.0, h.bottom() - ch - 24.0)
                    }
                }
            }
            _ => egui::pos2(screen.center().x - card_w / 2.0, screen.center().y - ch / 2.0),
        };
        let pos = match tut.card_pos {
            Some(c) => c.lerp(desired, k),
            None => desired,
        };
        tut.card_pos = Some(pos);

        let st = ease_out(tut.step_since.elapsed().as_secs_f32() / 0.32);
        let last = tut.step + 1 == STEPS.len();
        let shadow = ctx.style().visuals.window_shadow;
        let ic = self.icons;
        let mut action: Option<bool> = None; // true — дальше, false — пропустить
        let card = egui::Area::new(Id::new("tut-card"))
            .order(Order::Tooltip)
            .fixed_pos(pos + vec2(0.0, (1.0 - st) * 14.0))
            .interactable(tut.closing.is_none())
            .show(ctx, |ui| {
                ui.set_opacity(a * st);
                Frame::none()
                    .fill(p.surface_container_high)
                    .rounding(24.0)
                    .shadow(shadow)
                    .inner_margin(Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.set_width(card_w - 40.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Шаг {} из {}", tut.step + 1, STEPS.len()))
                                    .size(12.5)
                                    .color(p.on_surface_variant),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for i in (0..STEPS.len()).rev() {
                                    let cur = i == tut.step;
                                    let (r, _) = ui.allocate_exact_size(vec2(if cur { 18.0 } else { 6.0 }, 6.0), Sense::hover());
                                    let c = if i <= tut.step { p.primary } else { p.outline_variant };
                                    ui.painter().rect_filled(r, 3.0, c);
                                }
                            });
                        });
                        ui.add_space(2.0);
                        ui.label(RichText::new(step.title).font(egui::FontId::new(19.0, bold_family())).color(p.on_surface));
                        ui.label(RichText::new(step.text).size(14.5).color(p.on_surface_variant));
                        if step.wait != Wait::Next {
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                                cookie(ui.painter(), r.center(), 9.0, 7, 0.14, time * 1.6, p.primary);
                                ui.label(RichText::new("Сделайте это на подсвеченном месте").size(13.5).color(p.primary));
                            });
                        }
                        ui.add_space(10.0);
                        ui.allocate_ui_with_layout(vec2(ui.available_width(), 40.0), Layout::right_to_left(Align::Center), |ui| {
                            if step.wait == Wait::Next {
                                let label = if tut.step == 0 {
                                    "Начать"
                                } else if last {
                                    "Начать работу"
                                } else {
                                    "Далее"
                                };
                                if pill(ui, if last { ic.check } else { "" }, label, p.primary, p.on_primary, 40.0, 20.0, 0.0).clicked() {
                                    action = Some(true);
                                }
                            }
                            if !last && pill(ui, "", "Пропустить", Color32::TRANSPARENT, p.primary, 40.0, 20.0, 0.0).clicked() {
                                action = Some(false);
                            }
                        });
                    });
            });
        tut.card_h = card.response.rect.height();

        if tut.closing.is_none() {
            let d = &self.docs[self.active];
            let done = match step.wait {
                Wait::Next => false,
                Wait::NewDoc => self.ev_new_doc,
                Wait::Type => d.id == tut.doc0 && d.text.chars().count() >= tut.len0 + 3,
                Wait::View => self.settings.view != tut.view0,
                Wait::Settings => self.settings_open,
            };
            if ctx.input(|i| i.key_pressed(Key::Escape)) && !self.settings_open {
                action = Some(false);
            }
            match action {
                Some(true) => self.tut_advance(&mut tut),
                Some(false) => self.tut_finish(&mut tut),
                None if done => self.tut_advance(&mut tut),
                None => {}
            }
        }
        self.tutorial = Some(tut);
    }

    fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn snack(&mut self, msg: impl Into<String>) {
        self.snackbar = Some((msg.into(), Instant::now()));
    }

    fn new_doc(&mut self) {
        self.untitled += 1;
        let name = if self.untitled == 1 {
            "Без названия".to_string()
        } else {
            format!("Без названия {}", self.untitled)
        };
        let id = self.alloc_id();
        self.docs.push(Doc::new(id, None, name, String::new(), false));
        self.active = self.docs.len() - 1;
        self.ev_new_doc = true;
    }

    fn open_dialog(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .set_title("Открыть")
            .add_filter("Markdown и текст", &["md", "markdown", "txt"])
            .add_filter("Все файлы", &["*"]);
        if let Some(dir) = self.docs.get(self.active).and_then(|d| d.path.as_ref()).and_then(|p| p.parent()) {
            dlg = dlg.set_directory(dir);
        }
        if let Some(files) = dlg.pick_files() {
            for f in files {
                self.open_path(&f);
            }
        }
    }

    fn open_path(&mut self, path: &Path) {
        if let Some(i) = self.docs.iter().position(|d| d.path.as_deref() == Some(path)) {
            self.active = i;
            return;
        }
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut text = match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                };
                if text.starts_with('\u{feff}') {
                    text.remove(0);
                }
                let crlf = text.contains("\r\n");
                if crlf {
                    text = text.replace("\r\n", "\n");
                }
                let id = self.alloc_id();
                let doc = Doc::new(id, Some(path.to_path_buf()), String::new(), text, crlf);
                // Заменяем пустую/приветственную заметку без изменений
                let replace = self
                    .docs
                    .get(self.active)
                    .map(|d| d.path.is_none() && !d.dirty)
                    .unwrap_or(false);
                if replace {
                    self.docs[self.active] = doc;
                } else {
                    self.docs.push(doc);
                    self.active = self.docs.len() - 1;
                }
            }
            Err(e) => self.snack(format!("Не удалось открыть «{}»: {e}", path.display())),
        }
    }

    /// Сохраняет заметку. Возвращает `false`, если пользователь отменил или произошла ошибка.
    fn save_doc(&mut self, i: usize, save_as: bool) -> bool {
        let path = if save_as || self.docs[i].path.is_none() {
            let d = &self.docs[i];
            let mut name = d.title();
            if d.path.is_none() && !name.to_lowercase().ends_with(".md") {
                name.push_str(".md");
            }
            let mut dlg = rfd::FileDialog::new()
                .set_title("Сохранить как")
                .set_file_name(&name)
                .add_filter("Markdown", &["md", "markdown"])
                .add_filter("Текстовый файл", &["txt"])
                .add_filter("Все файлы", &["*"]);
            if let Some(dir) = d.path.as_ref().and_then(|p| p.parent()) {
                dlg = dlg.set_directory(dir);
            }
            match dlg.save_file() {
                Some(p) => p,
                None => return false,
            }
        } else {
            self.docs[i].path.clone().unwrap()
        };

        let d = &self.docs[i];
        let content = if d.crlf { d.text.replace('\n', "\r\n") } else { d.text.clone() };
        match std::fs::write(&path, content) {
            Ok(()) => {
                let d = &mut self.docs[i];
                d.path = Some(path);
                d.dirty = false;
                let t = d.title();
                self.snack(format!("Сохранено: {t}"));
                true
            }
            Err(e) => {
                self.snack(format!("Ошибка сохранения: {e}"));
                false
            }
        }
    }

    fn request_close(&mut self, i: usize) {
        if self.docs[i].dirty {
            self.pending = Some(Pending::CloseDoc(self.docs[i].id));
        } else {
            self.begin_close(i);
        }
    }

    /// Запускает анимацию закрытия; сама заметка удаляется в `process_closing`.
    fn begin_close(&mut self, i: usize) {
        if self.docs[i].closing.is_some() {
            return;
        }
        self.docs[i].closing = Some(Instant::now());
        if i == self.active {
            let next = (i + 1..self.docs.len())
                .chain((0..i).rev())
                .find(|&j| self.docs[j].closing.is_none());
            match next {
                Some(j) => self.active = j,
                None => self.new_doc(),
            }
        }
    }

    fn process_closing(&mut self, ctx: &egui::Context) {
        let mut i = self.docs.len();
        while i > 0 {
            i -= 1;
            if let Some(t) = self.docs[i].closing {
                if t.elapsed().as_secs_f32() > 0.24 {
                    self.remove_doc(i);
                } else {
                    ctx.request_repaint();
                }
            }
        }
    }

    fn remove_doc(&mut self, i: usize) {
        if i >= self.docs.len() {
            return;
        }
        self.docs.remove(i);
        if self.docs.is_empty() {
            self.new_doc();
        }
        if self.active > i {
            self.active -= 1;
        }
        self.active = self.active.min(self.docs.len() - 1);
    }

    // ---------------------------------------------------------------- ввод

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested())
            && !self.force_exit
            && self.docs.iter().any(|d| d.dirty)
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.pending = Some(Pending::Exit);
        }
    }

    fn handle_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for p in dropped {
            self.open_path(&p);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let sc = |m: Modifiers, k: Key| KeyboardShortcut::new(m, k);
        let hit = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));

        if hit(sc(cmd | Modifiers::SHIFT, Key::S)) {
            self.save_doc(self.active, true);
        }
        if hit(sc(cmd, Key::S)) {
            self.save_doc(self.active, false);
        }
        if hit(sc(cmd, Key::N)) {
            self.new_doc();
        }
        if hit(sc(cmd, Key::O)) {
            self.open_dialog();
        }
        if hit(sc(cmd, Key::W)) {
            self.request_close(self.active);
        }
        if hit(sc(cmd, Key::Tab)) && !self.docs.is_empty() {
            self.active = (self.active + 1) % self.docs.len();
        }
        if hit(sc(cmd, Key::E)) {
            self.settings.view = match self.settings.view {
                ViewMode::Edit => ViewMode::Split,
                ViewMode::Split => ViewMode::Preview,
                ViewMode::Preview => ViewMode::Edit,
            };
        }
        if self.settings.view != ViewMode::Preview {
            if hit(sc(cmd, Key::B)) {
                self.fmt_request = Some(Fmt::Wrap("**", "**", "жирный"));
            }
            if hit(sc(cmd, Key::I)) {
                self.fmt_request = Some(Fmt::Wrap("*", "*", "курсив"));
            }
            if hit(sc(cmd, Key::K)) {
                self.fmt_request = Some(Fmt::Wrap("[", "](https://)", "ссылка"));
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let d = &self.docs[self.active];
        let title = format!("{}{} — Notepad You", if d.dirty { "• " } else { "" }, d.title());
        if title != self.last_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }

    // ---------------------------------------------------------------- UI

    fn sidebar(&mut self, ctx: &egui::Context, p: &Palette) {
        let ic = self.icons;
        let modal = self.pending.is_some();
        SidePanel::left("drawer")
            .resizable(false)
            .exact_width(268.0)
            .show_separator_line(false)
            .frame(Frame::none().fill(p.surface).inner_margin(Margin { left: 12.0, right: 8.0, top: 14.0, bottom: 12.0 }))
            .show_animated(ctx, self.settings.sidebar, |ui| {
                if modal {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    // Логотип-печенька: при наведении поворачивается и чуть подрастает
                    let (r, resp) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
                    let h = ease_out(ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.35));
                    // При смене темы новый логотип проявляется поверх старого и делает оборот
                    let fade = self
                        .logo_prev
                        .as_ref()
                        .map(|(_, t)| ease_out(t.elapsed().as_secs_f32() / 0.5))
                        .unwrap_or(1.0);
                    let size = 42.0 + 4.0 * h + 5.0 * (fade * std::f32::consts::PI).sin();
                    let spin = h * 0.5 + (1.0 - fade) * -1.2;
                    let dst = Rect::from_center_size(r.center(), vec2(size, size));
                    if let Some((prev, _)) = &self.logo_prev {
                        egui::Image::new(egui::load::SizedTexture::new(prev.id(), vec2(size, size)))
                            .rotate(spin, vec2(0.5, 0.5))
                            .tint(Color32::WHITE.gamma_multiply(1.0 - fade))
                            .paint_at(ui, dst);
                    }
                    egui::Image::new(egui::load::SizedTexture::new(self.logo.id(), vec2(size, size)))
                        .rotate(spin, vec2(0.5, 0.5))
                        .tint(Color32::WHITE.gamma_multiply(fade))
                        .paint_at(ui, dst);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new("Notepad You").font(egui::FontId::new(22.0, bold_family())).color(p.primary));
                        ui.label(RichText::new("Material You · Markdown").size(12.5).color(p.on_surface_variant));
                    });
                });
                ui.add_space(16.0);

                let fab = pill(ui, ic.add, "Новая заметка", p.primary_container, p.on_primary_container, 56.0, 16.0, 0.0)
                    .on_hover_text("Ctrl+N");
                self.targets.insert("fab", fab.rect);
                if fab.clicked() {
                    self.new_doc();
                }
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(RichText::new("Заметки").size(13.0).color(p.on_surface_variant));
                });
                ui.add_space(2.0);

                let mut select = None;
                let mut close = None;
                let notes_top = ui.cursor().top();
                ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    let active = self.active;
                    for (i, d) in self.docs.iter().enumerate() {
                        // Появление: выезжает слева и раскрывается по высоте; закрытие — наоборот
                        let appear = ease_out(d.born.elapsed().as_secs_f32() / 0.32);
                        let vanish = d.closing.map(|t| ease_out(t.elapsed().as_secs_f32() / 0.22)).unwrap_or(0.0);
                        let k = appear * (1.0 - vanish);
                        if appear < 1.0 || d.closing.is_some() {
                            ui.ctx().request_repaint();
                        }
                        if k < 0.01 {
                            continue;
                        }
                        ui.scope(|ui| {
                        ui.set_opacity(k);
                        let (full, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0 * k), Sense::click());
                        let rect = Rect::from_center_size(full.center(), vec2(full.width(), 48.0))
                            .translate(vec2(-(1.0 - k) * 28.0, 0.0));
                        let close_rect = widgets::centered_rect(egui::pos2(rect.right() - 26.0, rect.center().y), 30.0);
                        let close_resp = ui.interact(close_rect, ui.id().with(("close", d.id)), Sense::click());
                        let hovered = (resp.hovered() || close_resp.hovered()) && d.closing.is_none();
                        let selected = i == active;
                        let sel = ease_out(ui.ctx().animate_bool_with_time(Id::new(("sel", d.id)), selected, 0.28));
                        let hov = ui.ctx().animate_bool_with_time(Id::new(("hov", d.id)), hovered, 0.15);
                        if ui.is_rect_visible(full) {
                            let painter = ui.painter_at(full.expand2(vec2(40.0, 0.0)));
                            if sel > 0.0 {
                                // Индикатор раскрывается от центра (как в навигации Material 3)
                                let w = rect.width() * egui::lerp(0.55..=1.0, sel);
                                let ind = Rect::from_center_size(rect.center(), vec2(w, rect.height()));
                                painter.rect_filled(ind, 24.0, p.secondary_container.gamma_multiply(sel));
                            }
                            if hov > 0.0 {
                                painter.rect_filled(rect, 24.0, p.on_surface.gamma_multiply(0.06 * hov));
                            }
                            let fg = mix(p.on_surface_variant, p.on_secondary_container, sel);
                            painter.text(
                                egui::pos2(rect.left() + 26.0, rect.center().y),
                                Align2::CENTER_CENTER,
                                ic.doc,
                                icon_font(16.0),
                                fg,
                            );
                            let font = if selected {
                                egui::FontId::new(14.5, bold_family())
                            } else {
                                egui::FontId::proportional(14.5)
                            };
                            let mut job = LayoutJob::simple_singleline(d.title(), font, fg);
                            job.wrap = TextWrapping {
                                max_width: rect.width() - 52.0 - 44.0,
                                max_rows: 1,
                                break_anywhere: true,
                                overflow_character: Some('…'),
                            };
                            let galley = ui.fonts(|f| f.layout_job(job));
                            let y = rect.center().y - galley.size().y / 2.0;
                            painter.galley(egui::pos2(rect.left() + 50.0, y), galley, fg);

                            if hov > 0.0 {
                                let ch = ui.ctx().animate_bool_with_time(
                                    Id::new(("close-h", d.id)),
                                    close_resp.hovered(),
                                    0.18,
                                );
                                if ch > 0.0 {
                                    cookie(&painter, close_rect.center(), 13.0 + 2.0 * ch, 8, 0.12, ch * 0.4, p.on_surface.gamma_multiply(0.12 * ch));
                                }
                                painter.text(close_rect.center(), Align2::CENTER_CENTER, ic.close, icon_font(11.0), fg.gamma_multiply(hov));
                            }
                            if d.dirty && hov < 1.0 {
                                painter.circle_filled(close_rect.center(), 4.5, p.primary.gamma_multiply(1.0 - hov));
                            }
                        }
                        if d.closing.is_none() {
                            if close_resp.clicked() {
                                close = Some(i);
                            } else if resp.clicked() {
                                select = Some(i);
                            }
                        }
                        let tip = d
                            .path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "Не сохранено".into());
                        resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip);
                        ui.add_space(2.0 * k);
                        });
                    }
                });
                // Область списка для обучения: от заголовка до последней заметки
                let n = self.docs.iter().filter(|d| d.closing.is_none()).count() as f32;
                let list = Rect::from_min_size(
                    egui::pos2(ui.min_rect().left(), notes_top),
                    vec2(ui.min_rect().width(), (n * 50.0).min(ui.max_rect().bottom() - notes_top)),
                );
                self.targets.insert("notes", list);
                if let Some(i) = select {
                    self.active = i;
                }
                if let Some(i) = close {
                    self.request_close(i);
                }
            });
    }

    fn top_bar(&mut self, ctx: &egui::Context, p: &Palette) {
        let ic = self.icons;
        let modal = self.pending.is_some();
        TopBottomPanel::top("top")
            .show_separator_line(false)
            .frame(Frame::none().fill(p.surface).inner_margin(Margin { left: 8.0, right: 16.0, top: 10.0, bottom: 6.0 }))
            .show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    if icon_button(ui, ic.menu, "Показать/скрыть список заметок", p).clicked() {
                        self.settings.sidebar = !self.settings.sidebar;
                    }
                    ui.add_space(4.0);
                    let (title, dirty) = {
                        let d = &self.docs[self.active];
                        (d.title(), d.dirty)
                    };
                    ui.label(RichText::new(title).font(egui::FontId::new(19.0, bold_family())).color(p.on_surface));
                    if dirty {
                        ui.label(RichText::new("●").size(11.0).color(p.primary)).on_hover_text("Есть несохранённые изменения");
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.settings_menu(ui, p);
                        let (icon, tip) = if self.settings.dark { (ic.sun, "Светлая тема") } else { (ic.moon, "Тёмная тема") };
                        if icon_button(ui, icon, tip, p).clicked() {
                            self.settings.dark = !self.settings.dark;
                        }
                        ui.add_space(6.0);
                        if icon_button(ui, ic.save_as, "Сохранить как… (Ctrl+Shift+S)", p).clicked() {
                            self.save_doc(self.active, true);
                        }
                        let save = icon_button(ui, ic.save, "Сохранить (Ctrl+S)", p);
                        self.targets.insert("save", save.rect);
                        if save.clicked() {
                            self.save_doc(self.active, false);
                        }
                        if icon_button(ui, ic.open, "Открыть (Ctrl+O)", p).clicked() {
                            self.open_dialog();
                        }
                        ui.add_space(10.0);
                        let seg = ui.scope(|ui| {
                            segmented(
                                ui,
                                &mut self.settings.view,
                                &[(ViewMode::Edit, "Редактор"), (ViewMode::Split, "Обе"), (ViewMode::Preview, "Просмотр")],
                                ic.check,
                                p,
                            );
                        });
                        self.targets.insert("view", seg.response.rect);
                    });
                });
            });
    }

    fn settings_menu(&mut self, ui: &mut Ui, p: &Palette) {
        let ic = self.icons;
        let resp = icon_button(ui, ic.settings, "Настройки", p);
        self.targets.insert("settings", resp.rect);
        if resp.clicked() {
            self.settings_open = !self.settings_open;
        }
        let ctx = ui.ctx().clone();
        // Панель плавно выезжает сверху и проявляется
        let t = ease_out(ctx.animate_bool_with_time(Id::new("settings-anim"), self.settings_open, 0.24));
        if t <= 0.0 {
            return;
        }
        let width = 340.0;
        let pos = egui::pos2(resp.rect.right() - width, resp.rect.bottom() + 8.0 - (1.0 - t) * 14.0);
        let shadow = ctx.style().visuals.popup_shadow;
        let area = egui::Area::new(Id::new("settings-area"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .interactable(self.settings_open)
            .show(&ctx, |ui| {
                ui.set_opacity(t);
                Frame::none()
                    .fill(p.surface_container_high)
                    .rounding(28.0)
                    .shadow(shadow)
                    .inner_margin(Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.set_width(width - 40.0);
                        self.settings_contents(ui, p);
                    });
            });
        if t > 0.95 && self.settings_open {
            self.targets.insert("settings_panel", area.response.rect);
        }
        if self.settings_open && !resp.clicked() {
            let rect = area.response.rect;
            let outside = ctx.input(|i| {
                i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|pt| !rect.contains(pt) && !resp.rect.contains(pt))
            });
            if outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.settings_open = false;
            }
        }
    }

    fn settings_contents(&mut self, ui: &mut Ui, p: &Palette) {
        let ic = self.icons;
        {
            ui.label(RichText::new("Настройки").font(egui::FontId::new(20.0, bold_family())));
            ui.add_space(2.0);
            ui.label(RichText::new("Цвет оформления").font(egui::FontId::new(14.5, bold_family())).color(p.on_surface_variant));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for seed in Seed::ALL {
                    let c = Palette::new(seed, self.settings.dark).primary;
                    if widgets::color_dot(ui, c, seed == self.settings.seed, ic.check, p)
                        .on_hover_text(seed.name())
                        .clicked()
                    {
                        self.settings.seed = seed;
                    }
                }
            });
            ui.add_space(6.0);
            let row = |ui: &mut Ui, label: &str, value: &mut bool| {
                ui.allocate_ui_with_layout(vec2(ui.available_width(), 36.0), Layout::left_to_right(Align::Center), |ui| {
                    ui.label(label);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        switch(ui, value, p);
                    });
                });
            };
            row(ui, "Тёмная тема", &mut self.settings.dark);
            row(ui, "Моноширинный шрифт", &mut self.settings.mono);
            ui.add_space(4.0);
            ui.label(RichText::new("Шрифт").font(egui::FontId::new(14.5, bold_family())).color(p.on_surface_variant));
            ui.horizontal(|ui| {
                segmented(
                    ui,
                    &mut self.settings.font,
                    &[(UiFont::Nunito, UiFont::Nunito.name()), (UiFont::Segoe, UiFont::Segoe.name())],
                    ic.check,
                    p,
                );
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Размер текста").font(egui::FontId::new(14.5, bold_family())).color(p.on_surface_variant));
            ui.spacing_mut().slider_width = 230.0;
            ui.add(egui::Slider::new(&mut self.settings.font_size, 11.0..=30.0).step_by(1.0).suffix(" px"));
            ui.add_space(8.0);
            if pill(ui, "", "Пройти обучение заново", p.secondary_container, p.on_secondary_container, 38.0, 19.0, 0.0).clicked() {
                self.start_tutorial();
            }
            ui.add_space(6.0);
            ui.label(RichText::new("Notepad You 1.1 · Rust + egui").size(11.5).color(p.on_surface_variant));
        }
    }

    fn status_bar(&mut self, ctx: &egui::Context, p: &Palette) {
        let d = &self.docs[self.active];
        TopBottomPanel::bottom("status")
            .show_separator_line(false)
            .frame(Frame::none().fill(p.surface).inner_margin(Margin::symmetric(22.0, 7.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let small = |s: String| RichText::new(s).size(12.5).color(p.on_surface_variant);
                    let path = d.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "Не сохранено".into());
                    ui.label(small(path));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let lines = d.text.lines().count().max(1);
                        let words = d.text.split_whitespace().count();
                        let chars = d.text.chars().count();
                        let kind = match d.path.as_ref().and_then(|p| p.extension()).and_then(|e| e.to_str()) {
                            Some(e) if e.eq_ignore_ascii_case("txt") => "Текст",
                            _ => "Markdown",
                        };
                        let eol = if d.crlf { "CRLF" } else { "LF" };
                        ui.label(small(format!(
                            "{kind}  ·  UTF-8 {eol}  ·  Строк: {lines}  ·  Слов: {words}  ·  Символов: {chars}"
                        )));
                    });
                });
            });
    }

    fn central(&mut self, ctx: &egui::Context, p: &Palette) {
        let ic = self.icons;
        let modal = self.pending.is_some();
        let left = if self.settings.sidebar { 4.0 } else { 16.0 };
        let (size, mono, view) = (self.settings.font_size, self.settings.mono, self.settings.view);
        let mut fmt = self.fmt_request.take();
        // Плавная смена содержимого при переключении заметки или режима
        let key = (self.docs[self.active].id, view);
        if key != self.content_key {
            self.content_key = key;
            self.content_since = Instant::now();
        }
        let ct = ease_out(self.content_since.elapsed().as_secs_f32() / 0.3);
        if ct < 1.0 {
            ctx.request_repaint();
        }
        CentralPanel::default()
            .frame(Frame::none().fill(p.surface).inner_margin(Margin { left, right: 16.0, top: 2.0, bottom: 0.0 }))
            .show(ctx, |ui| {
                if modal {
                    ui.disable();
                }
                Frame::none()
                    .fill(p.surface_container_low)
                    .rounding(24.0)
                    .inner_margin(Margin::symmetric(16.0, 12.0))
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        if view != ViewMode::Preview {
                            let tb = toolbar(ui, &ic, p, &mut fmt);
                            self.targets.insert("toolbar", tb);
                            ui.add_space(2.0);
                            let r = ui.available_rect_before_wrap();
                            ui.painter().hline(r.x_range(), r.top(), egui::Stroke::new(1.0_f32, p.outline_variant));
                            ui.add_space(10.0);
                        }
                        let doc = &mut self.docs[self.active];
                        let emoji = &*self.emoji;
                        let mut editor_rect = None;
                        ui.scope(|ui| {
                            ui.set_opacity(ct);
                            ui.add_space((1.0 - ct) * 16.0);
                            match view {
                                ViewMode::Edit => editor_rect = Some(editor_ui(ui, doc, p, size, mono, fmt, emoji)),
                                ViewMode::Preview => preview_ui(ui, doc, p, size, false, emoji),
                                ViewMode::Split => {
                                    ui.columns(2, |cols| {
                                        editor_rect = Some(editor_ui(&mut cols[0], doc, p, size, mono, fmt, emoji));
                                        preview_ui(&mut cols[1], doc, p, size, true, emoji);
                                    });
                                }
                            }
                        });
                        if let Some(r) = editor_rect {
                            self.targets.insert("editor", r);
                        }
                    });
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context, p: &Palette) {
        let open = self.pending.is_some();
        // Диалог проявляется и всплывает снизу, фон плавно затемняется
        let t = ease_out(ctx.animate_bool_with_time(Id::new("dialog-anim"), open, 0.24));
        if t <= 0.0 {
            return;
        }

        if let Some(pending) = self.pending {
            self.dialog_text = match pending {
                Pending::CloseDoc(id) => {
                    let name = self.docs.iter().find(|d| d.id == id).map(|d| d.title()).unwrap_or_default();
                    ("Сохранить изменения?".to_string(), format!("В заметке «{name}» есть несохранённые изменения."))
                }
                Pending::Exit => {
                    let n = self.docs.iter().filter(|d| d.dirty).count();
                    ("Сохранить перед выходом?".to_string(), format!("Несохранённых заметок: {n}. Изменения будут потеряны."))
                }
            };
        }

        ctx.layer_painter(LayerId::new(Order::PanelResizeLine, Id::new("scrim")))
            .rect_filled(ctx.screen_rect(), 0.0, Color32::from_black_alpha((110.0 * t) as u8));

        let (title, body) = self.dialog_text.clone();
        let shadow = ctx.style().visuals.window_shadow;
        let mut action: Option<u8> = None; // 0 — сохранить, 1 — не сохранять, 2 — отмена
        egui::Area::new(Id::new("confirm"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, (1.0 - t) * 28.0))
            .interactable(open)
            .show(ctx, |ui| {
              ui.set_opacity(t);
              Frame::none()
                .fill(p.surface_container_high)
                .rounding(28.0)
                .shadow(shadow)
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                ui.set_width(360.0);
                ui.label(RichText::new(title).font(egui::FontId::new(22.0, bold_family())).color(p.on_surface));
                ui.add_space(6.0);
                ui.label(RichText::new(body).color(p.on_surface_variant));
                ui.add_space(18.0);
                ui.allocate_ui_with_layout(vec2(ui.available_width(), 40.0), Layout::right_to_left(Align::Center), |ui| {
                    if pill(ui, "", "Сохранить", p.primary, p.on_primary, 40.0, 20.0, 0.0).clicked() {
                        action = Some(0);
                    }
                    if pill(ui, "", "Не сохранять", Color32::TRANSPARENT, p.primary, 40.0, 20.0, 0.0).clicked() {
                        action = Some(1);
                    }
                    if pill(ui, "", "Отмена", Color32::TRANSPARENT, p.primary, 40.0, 20.0, 0.0).clicked() {
                        action = Some(2);
                    }
                });
                });
            });
        let Some(pending) = self.pending else { return };
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            action = Some(2);
        }

        let Some(action) = action else { return };
        self.pending = None;
        let doc_index = |app: &Self, id: u64| app.docs.iter().position(|d| d.id == id);
        match (pending, action) {
            (Pending::CloseDoc(id), 0) => {
                if let Some(i) = doc_index(self, id) {
                    if self.save_doc(i, false) {
                        self.begin_close(i);
                    }
                }
            }
            (Pending::CloseDoc(id), 1) => {
                if let Some(i) = doc_index(self, id) {
                    self.begin_close(i);
                }
            }
            (Pending::Exit, 0) => {
                for i in 0..self.docs.len() {
                    if self.docs[i].dirty && !self.save_doc(i, false) {
                        return; // пользователь отменил — не выходим
                    }
                }
                self.force_exit = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            (Pending::Exit, 1) => {
                self.force_exit = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            _ => {}
        }
    }

    fn overlays(&mut self, ctx: &egui::Context, p: &Palette) {
        // Snackbar
        if let Some((msg, t)) = &self.snackbar {
            let age = t.elapsed().as_secs_f32();
            if age > 2.8 {
                self.snackbar = None;
            } else {
                let enter = ease_out(age / 0.3);
                let fade = ((2.8 - age) / 0.3).clamp(0.0, 1.0).min(enter);
                egui::Area::new(Id::new("snackbar"))
                    .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -46.0 + (1.0 - enter) * 36.0))
                    .order(Order::Foreground)
                    .interactable(false)
                    .show(ctx, |ui| {
                        Frame::none()
                            .fill(p.inverse_surface.gamma_multiply(fade))
                            .rounding(8.0)
                            .inner_margin(Margin::symmetric(18.0, 12.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg.as_str()).color(p.inverse_on_surface.gamma_multiply(fade)));
                            });
                    });
                ctx.request_repaint();
            }
        }

        // Подсказка при перетаскивании файлов
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let screen = ctx.screen_rect();
            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("drop")));
            painter.rect_filled(screen, 0.0, p.surface.gamma_multiply(0.85));
            painter.rect_stroke(screen.shrink(18.0), 28.0, egui::Stroke::new(3.0_f32, p.primary));
            painter.text(
                screen.center(),
                Align2::CENTER_CENTER,
                "Отпустите файл, чтобы открыть",
                egui::FontId::new(24.0, bold_family()),
                p.primary,
            );
        }
    }
}

fn toolbar(ui: &mut Ui, ic: &Icons, p: &Palette, fmt: &mut Option<Fmt>) -> Rect {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let bold = egui::FontId::new(14.0, bold_family());
        let text = egui::FontId::proportional(16.0);
        let icon = icon_font(15.0);
        let mut b = |ui: &mut Ui, label: &str, font: &egui::FontId, tip: &str, f: Fmt| {
            if tool_button(ui, label, font.clone(), tip, p).clicked() {
                *fmt = Some(f);
            }
        };
        b(ui, "H1", &bold, "Заголовок 1", Fmt::Line("# "));
        b(ui, "H2", &bold, "Заголовок 2", Fmt::Line("## "));
        b(ui, "H3", &bold, "Заголовок 3", Fmt::Line("### "));
        widgets::vertical_divider(ui, p);
        b(ui, ic.bold, &icon, "Жирный (Ctrl+B)", Fmt::Wrap("**", "**", "жирный"));
        b(ui, ic.italic, &icon, "Курсив (Ctrl+I)", Fmt::Wrap("*", "*", "курсив"));
        b(ui, "S̶", &text, "Зачёркнутый", Fmt::Wrap("~~", "~~", "зачёркнутый"));
        b(ui, ic.code, &icon, "Код в строке", Fmt::Wrap("`", "`", "код"));
        b(ui, ic.link, &icon, "Ссылка (Ctrl+K)", Fmt::Wrap("[", "](https://)", "ссылка"));
        widgets::vertical_divider(ui, p);
        b(ui, ic.list, &icon, "Маркированный список", Fmt::Line("- "));
        b(ui, "1.", &bold, "Нумерованный список", Fmt::Line("1. "));
        b(ui, "☑", &text, "Список задач", Fmt::Line("- [ ] "));
        b(ui, "❝", &text, "Цитата", Fmt::Line("> "));
        widgets::vertical_divider(ui, p);
        b(ui, "{ }", &bold, "Блок кода", Fmt::Wrap("```\n", "\n```", "код"));
        b(
            ui,
            "⊞",
            &text,
            "Таблица",
            Fmt::Insert("\n| Столбец 1 | Столбец 2 |\n|---|---|\n| Ячейка | Ячейка |\n"),
        );
        b(ui, "―", &text, "Разделитель", Fmt::Insert("\n---\n"));
    })
    .response
    .rect
}

fn apply_fmt(ctx: &egui::Context, id: Id, doc: &mut Doc, fmt: Fmt) {
    let len = doc.text.chars().count();
    let (a, b) = TextEdit::load_state(ctx, id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| {
            let (x, y) = (r.primary.index, r.secondary.index);
            (x.min(y).min(len), x.max(y).min(len))
        })
        .unwrap_or((len, len));
    let (na, nb) = fmt.apply(&mut doc.text, a, b);
    doc.dirty = true;
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(na), CCursor::new(nb))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

fn editor_ui(
    ui: &mut Ui,
    doc: &mut Doc,
    p: &Palette,
    size: f32,
    mono: bool,
    fmt: Option<Fmt>,
    emoji: &crate::emoji::Emoji,
) -> Rect {
    let id = Id::new(("editor", doc.id));
    if let Some(f) = fmt {
        apply_fmt(ui.ctx(), id, doc, f);
    }
    let pal = *p;
    let mut layouter = move |ui: &Ui, text: &str, wrap: f32| {
        let mut job = highlight::layout_job(text, &pal, size, mono);
        job.wrap.max_width = wrap;
        ui.fonts(|f| f.layout_job(job))
    };
    ScrollArea::vertical()
        .id_salt(("editor-scroll", doc.id))
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let out = TextEdit::multiline(&mut doc.text)
                .id(id)
                .frame(false)
                .lock_focus(true)
                .desired_width(f32::INFINITY)
                .min_size(ui.available_size())
                .hint_text("Начните писать… Поддерживается Markdown.")
                .layouter(&mut layouter)
                .show(ui);
            if out.response.changed() {
                doc.dirty = true;
            }
            paint_editor_emoji(ui, &doc.text, &out.galley, out.galley_pos, size, emoji);
        })
        .inner_rect
}

/// Рисует цветные смайлики поверх прозрачных символов в редакторе.
fn paint_editor_emoji(
    ui: &Ui,
    text: &str,
    galley: &egui::Galley,
    origin: Pos2,
    size: f32,
    emoji: &crate::emoji::Emoji,
) {
    let found = crate::emoji::clusters(text);
    if found.is_empty() {
        return;
    }
    let clip = ui.clip_rect();
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let mut char_idx = 0;
    let mut last = 0;
    for (a, b) in found {
        char_idx += text[last..a].chars().count();
        last = a;
        let r0 = galley.pos_from_ccursor(CCursor::new(char_idx));
        let r1 = galley.pos_from_ccursor(CCursor::new(char_idx + 1));
        let same_row = (r1.min.y - r0.min.y).abs() < 1.0;
        let w = if same_row { (r1.min.x - r0.min.x).max(4.0) } else { size };
        let center = origin + vec2(r0.min.x + w / 2.0, r0.center().y);
        if !clip.expand(size).contains(center) {
            continue;
        }
        if let Some((tex, aspect)) = emoji.texture(ui.ctx(), &text[a..b]) {
            let h = (size * 1.15).min(r0.height() * 1.05);
            let rect = Rect::from_center_size(center, vec2(h * aspect.x.min(2.5), h));
            ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
        }
    }
}

fn preview_ui(ui: &mut Ui, doc: &mut Doc, p: &Palette, size: f32, card: bool, emoji: &crate::emoji::Emoji) {
    let frame = if card {
        Frame::none().fill(p.surface_container).rounding(16.0).inner_margin(Margin::same(16.0))
    } else {
        Frame::none().inner_margin(Margin::symmetric(8.0, 4.0))
    };
    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        {
            let st = ui.style_mut();
            st.text_styles.insert(TextStyle::Body, egui::FontId::proportional(size));
            st.text_styles.insert(TextStyle::Heading, egui::FontId::new(size * 1.6, bold_family()));
            st.text_styles.insert(TextStyle::Monospace, egui::FontId::monospace(size * 0.9));
        }
        ScrollArea::vertical()
            .id_salt(("preview-scroll", doc.id))
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if doc.text.trim().is_empty() {
                    ui.label(RichText::new("Здесь появится предпросмотр Markdown").color(p.on_surface_variant).italics());
                } else {
                    // Смайлики → цветные картинки, «- [x]» → квадратные цветные галочки
                    let md = crate::emoji::preprocess(&doc.text, size, p.primary, p.on_primary, p.outline, emoji, ui.ctx());
                    CommonMarkViewer::new().show(ui, &mut doc.cache, &md);
                }
            });
    });
}

impl eframe::App for NotepadApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Плавный переход цветов при смене темы / тёмного режима
        let target = Palette::new(self.settings.seed, self.settings.dark);
        let key = (self.settings.seed, self.settings.dark);
        if self.applied != Some(key) {
            if self.applied.is_some() {
                self.pal_from = self.pal_cur;
                self.pal_since = Some(Instant::now());
            }
            self.applied = Some(key);
        }
        let p = match self.pal_since {
            Some(s) => {
                let t = ease_out(s.elapsed().as_secs_f32() / 0.45);
                if t >= 1.0 {
                    self.pal_since = None;
                    target
                } else {
                    ctx.request_repaint();
                    Palette::lerp(&self.pal_from, &target, t)
                }
            }
            None => target,
        };
        self.pal_cur = p;
        if ctx.style().visuals.panel_fill != p.surface || ctx.style().visuals.dark_mode != self.settings.dark {
            theme::apply(ctx, &p, self.settings.dark);
        }
        // Иконка в цветах темы: логотип в боковой панели и значок окна / панели задач
        if self.logo_seed != self.settings.seed {
            let new = make_logo(ctx, self.settings.seed);
            let old = std::mem::replace(&mut self.logo, new);
            self.logo_prev = Some((old, Instant::now()));
            self.logo_seed = self.settings.seed;
            set_window_icon(frame, self.settings.seed);
        }
        if !self.window_icon_sent {
            self.window_icon_sent = set_window_icon(frame, self.settings.seed);
        }
        if let Some((_, t)) = &self.logo_prev {
            if t.elapsed().as_secs_f32() > 0.5 {
                self.logo_prev = None;
            } else {
                ctx.request_repaint();
            }
        }
        if self.fonts_applied != self.settings.font {
            theme::setup_fonts(ctx, self.settings.font);
            self.fonts_applied = self.settings.font;
        }

        self.targets.clear();
        self.process_closing(ctx);
        self.handle_close_request(ctx);
        self.handle_files(ctx);
        if self.pending.is_none() {
            self.handle_shortcuts(ctx);
        }
        self.update_title(ctx);

        self.sidebar(ctx, &p);
        self.top_bar(ctx, &p);
        self.status_bar(ctx, &p);
        self.central(ctx, &p);
        self.dialogs(ctx, &p);
        self.overlays(ctx, &p);
        self.tutorial_ui(ctx, &p);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let with_path: Vec<usize> = (0..self.docs.len()).filter(|&i| self.docs[i].path.is_some()).collect();
        self.settings.session = with_path.iter().filter_map(|&i| self.docs[i].path.clone()).collect();
        self.settings.active = with_path.iter().position(|&i| i == self.active).unwrap_or(0);
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }
}
