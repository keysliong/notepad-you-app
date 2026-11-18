//! Установщик Notepad You в стиле Material You.
//! Ставит программу для текущего пользователя (без прав администратора):
//! %LOCALAPPDATA%\Programs\Notepad You, ярлыки, «Открыть с помощью», запись в «Установленные приложения».
//! Тот же exe, запущенный с `--uninstall`, удаляет программу.

#![windows_subsystem = "windows"]
#![allow(dead_code)]

#[path = "../../src/icon.rs"]
mod icon;
#[path = "../../src/theme.rs"]
mod theme;
#[path = "../../src/widgets.rs"]
mod widgets;

use eframe::egui::{
    self, pos2, vec2, Align, Align2, CentralPanel, Color32, FontId, Frame, Id, Layout, Margin,
    Painter, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder, ViewportCommand,
};
use std::f32::consts::{PI, TAU};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::{bold_family, Palette, Seed, UiFont};
use widgets::{cookie, ease_out, icon_button, icon_font, pill, switch};
use winreg::{enums::*, RegKey};

/// Сама программа, вшитая в установщик.
const APP_EXE: &[u8] = include_bytes!("../../target/release/NotepadYou.exe");
const VERSION: &str = "1.1.0";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\NotepadYou";
const NO_WINDOW: u32 = 0x0800_0000;

fn main() -> eframe::Result<()> {
    // Режим удаления: флаг --uninstall или имя файла вида «NotepadYou-Uninstall.exe» / «Uninstall.exe»
    let by_name = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .is_some_and(|s| s.contains("uninstall") || s.contains("удал"));
    let uninstall = by_name || std::env::args().any(|a| a == "--uninstall");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(if uninstall { "Удаление Notepad You" } else { "Установка Notepad You" })
            .with_inner_size([780.0, 500.0])
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(egui::IconData { rgba: icon::icon_rgba(64), width: 64, height: 64 }),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Notepad You Setup",
        options,
        Box::new(move |cc| Ok(Box::new(Setup::new(cc, uninstall)))),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Welcome,
    Options,
    Progress,
    Done,
    Error,
    UninstallConfirm,
    UninstallDone,
    NotInstalled,
}

#[derive(Default)]
struct Shared {
    progress: f32,
    status: String,
    finished: bool,
    error: Option<String>,
}

#[derive(Clone)]
struct Opts {
    dir: PathBuf,
    desktop: bool,
    start_menu: bool,
    assoc: bool,
    remove_settings: bool,
}

struct Setup {
    logo: egui::TextureHandle,
    page: Page,
    page_since: Instant,
    page_dir: f32,
    pal: Palette,
    has_icons: bool,
    dir: String,
    desktop: bool,
    start_menu: bool,
    assoc: bool,
    launch: bool,
    remove_settings: bool,
    uninstall: bool,
    already: bool,
    shared: Arc<Mutex<Shared>>,
    shown_progress: f32,
    error: String,
}

impl Setup {
    fn new(cc: &eframe::CreationContext<'_>, uninstall: bool) -> Self {
        let dark = system_dark();
        let pal = Palette::new(Seed::Lavender, dark);
        let logo = cc.egui_ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_unmultiplied([256, 256], &icon::icon_rgba(256)),
            egui::TextureOptions::LINEAR,
        );
        let has_icons = theme::setup_fonts(&cc.egui_ctx, UiFont::Nunito);
        theme::apply(&cc.egui_ctx, &pal, dark);

        let installed = installed_dir();
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
        let dir = if uninstall {
            // Сначала — запись в реестре, потом папка рядом с удалятором, потом папка по умолчанию
            installed
                .clone()
                .filter(|d| d.join("NotepadYou.exe").exists())
                .or_else(|| exe_dir.filter(|d| d.join("NotepadYou.exe").exists()))
                .unwrap_or_else(default_dir)
        } else {
            installed.clone().unwrap_or_else(default_dir)
        };
        let present = dir.join("NotepadYou.exe").exists();
        let first_page = match (uninstall, present) {
            (false, _) => Page::Welcome,
            (true, true) => Page::UninstallConfirm,
            (true, false) => Page::NotInstalled,
        };
        Self {
            logo,
            page: first_page,
            page_since: Instant::now(),
            page_dir: 1.0,
            pal,
            has_icons,
            dir: dir.display().to_string(),
            desktop: true,
            start_menu: true,
            assoc: true,
            launch: true,
            remove_settings: false,
            uninstall,
            already: if uninstall { present } else { installed.is_some_and(|d| d.join("NotepadYou.exe").exists()) },
            shared: Arc::new(Mutex::new(Shared::default())),
            shown_progress: 0.0,
            error: String::new(),
        }
    }

    fn go(&mut self, page: Page, dir: f32) {
        self.page = page;
        self.page_dir = dir;
        self.page_since = Instant::now();
    }

    fn opts(&self) -> Opts {
        Opts {
            dir: PathBuf::from(self.dir.trim()),
            desktop: self.desktop,
            start_menu: self.start_menu,
            assoc: self.assoc,
            remove_settings: self.remove_settings,
        }
    }

    fn start_work(&mut self) {
        *self.shared.lock().unwrap() = Shared::default();
        self.shown_progress = 0.0;
        let opts = self.opts();
        let sh = self.shared.clone();
        let uninstall = self.uninstall;
        std::thread::spawn(move || {
            let res = if uninstall { do_uninstall(&opts, &sh) } else { do_install(&opts, &sh) };
            let mut s = sh.lock().unwrap();
            s.finished = true;
            if let Err(e) = res {
                s.error = Some(e);
            }
        });
        self.go(Page::Progress, 1.0);
    }

    fn icon(&self, mdl2: &'static str, fallback: &'static str) -> &'static str {
        if self.has_icons {
            mdl2
        } else {
            fallback
        }
    }

    // ----------------------------------------------------------------- страницы

    fn page_welcome(&mut self, ui: &mut Ui, content: Rect) {
        let p = self.pal;
        let el = self.page_since.elapsed().as_secs_f32();
        let chip_text = if self.already {
            format!("Уже установлен · обновление до {VERSION}")
        } else {
            format!("Установщик · версия {VERSION}")
        };
        chip(ui, &p, &chip_text);
        ui.add_space(10.0);
        ui.label(RichText::new("Добро пожаловать!").font(FontId::new(32.0, bold_family())).color(p.on_surface));
        ui.label(
            RichText::new("Сейчас мы установим Notepad You — красивый блокнот в стиле Material You с поддержкой Markdown.")
                .size(15.5)
                .color(p.on_surface_variant),
        );
        ui.add_space(14.0);
        let features = [
            (self.icon("\u{E8A5}", "•"), "Markdown с живым предпросмотром"),
            (self.icon("\u{E790}", "•"), "7 цветовых тем и тёмный режим"),
            (self.icon("\u{E734}", "•"), "Анимации и кнопки-печеньки"),
            (self.icon("\u{E7BE}", "•"), "Обучение при первом запуске"),
        ];
        for (i, (ic, text)) in features.iter().enumerate() {
            let t = ease_out((el - 0.15 - i as f32 * 0.09) / 0.4);
            feature_row(ui, &p, ic, text, t);
        }
        let mut bar = bottom_bar(ui, content);
        let label = if self.already { "Обновить" } else { "Установить" };
        if pill(&mut bar, self.icon("\u{E896}", ""), label, p.primary, p.on_primary, 48.0, 24.0, 150.0).clicked() {
            self.start_work();
        }
        if pill(&mut bar, "", "Параметры", p.secondary_container, p.on_secondary_container, 48.0, 24.0, 0.0).clicked() {
            self.go(Page::Options, 1.0);
        }
    }

    fn page_options(&mut self, ui: &mut Ui, content: Rect) {
        let p = self.pal;
        ui.label(RichText::new("Параметры установки").font(FontId::new(28.0, bold_family())).color(p.on_surface));
        ui.add_space(8.0);
        ui.label(RichText::new("Папка установки").font(FontId::new(14.5, bold_family())).color(p.on_surface_variant));
        ui.horizontal(|ui| {
            let w = ui.available_width() - 120.0;
            ui.add(
                egui::TextEdit::singleline(&mut self.dir)
                    .desired_width(w)
                    .margin(Margin::symmetric(12.0, 10.0))
                    .font(FontId::proportional(14.5)),
            );
            if pill(ui, "", "Обзор…", p.secondary_container, p.on_secondary_container, 40.0, 20.0, 100.0).clicked() {
                let mut dlg = rfd::FileDialog::new().set_title("Куда установить Notepad You?");
                if let Some(parent) = Path::new(self.dir.trim()).parent() {
                    dlg = dlg.set_directory(parent);
                }
                if let Some(folder) = dlg.pick_folder() {
                    let dir = if folder.ends_with("Notepad You") { folder } else { folder.join("Notepad You") };
                    self.dir = dir.display().to_string();
                }
            }
        });
        ui.add_space(10.0);
        switch_row(ui, &p, "Ярлык на рабочем столе", &mut self.desktop);
        switch_row(ui, &p, "Ярлык в меню «Пуск»", &mut self.start_menu);
        switch_row(ui, &p, "Открывать .md и .txt через Notepad You", &mut self.assoc);
        ui.add_space(4.0);
        let mb = APP_EXE.len() as f32 / 1024.0 / 1024.0;
        ui.label(RichText::new(format!("Потребуется около {mb:.0} МБ. Права администратора не нужны.")).size(13.0).color(p.on_surface_variant));

        let mut bar = bottom_bar(ui, content);
        let ok = !self.dir.trim().is_empty();
        if pill(&mut bar, self.icon("\u{E896}", ""), "Установить", p.primary, p.on_primary, 48.0, 24.0, 150.0).clicked() && ok {
            self.start_work();
        }
        if pill(&mut bar, "", "Назад", Color32::TRANSPARENT, p.primary, 48.0, 24.0, 0.0).clicked() {
            self.go(Page::Welcome, -1.0);
        }
    }

    fn page_progress(&mut self, ui: &mut Ui, time: f32) {
        let p = self.pal;
        let (target, status) = {
            let s = self.shared.lock().unwrap();
            (s.progress, s.status.clone())
        };
        let dt = ui.ctx().input(|i| i.stable_dt).min(0.05);
        self.shown_progress += (target - self.shown_progress) * (1.0 - (-dt * 6.0).exp());
        let dots = ".".repeat(1 + (time * 2.5) as usize % 3);
        let title = if self.uninstall { "Удаляем" } else { "Устанавливаем" };
        ui.label(RichText::new(format!("{title}{dots}")).font(FontId::new(30.0, bold_family())).color(p.on_surface));
        ui.label(RichText::new(status).size(15.0).color(p.on_surface_variant));
        ui.add_space(18.0);
        ui.label(
            RichText::new(format!("{:.0}%", self.shown_progress * 100.0))
                .font(FontId::new(44.0, bold_family()))
                .color(p.primary),
        );
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
        wavy_bar(ui.painter(), bar, self.shown_progress, time, &p);
        ui.add_space(16.0);

        let steps: &[(&str, f32)] = if self.uninstall {
            &[("Удаление ярлыков", 0.35), ("Очистка реестра", 0.6), ("Удаление файлов", 1.0)]
        } else {
            &[("Копирование файлов", 0.6), ("Создание ярлыков", 0.82), ("Регистрация в системе", 1.0)]
        };
        let mut prev = 0.0;
        for (name, until) in steps {
            let state = if self.shown_progress >= *until - 0.005 {
                2
            } else if self.shown_progress >= prev {
                1
            } else {
                0
            };
            step_row(ui, &p, name, state, time, self.icon("\u{E73E}", "✔"));
            prev = *until;
        }
    }

    fn page_done(&mut self, ui: &mut Ui, content: Rect, ctx: &egui::Context) {
        let p = self.pal;
        let el = self.page_since.elapsed().as_secs_f32();
        let pop = back_out((el / 0.5).clamp(0.0, 1.0));
        ui.label(
            RichText::new("Готово!")
                .font(FontId::new(26.0 + 10.0 * pop, bold_family()))
                .color(p.on_surface),
        );
        ui.label(
            RichText::new("Notepad You установлен и готов к работе. При первом запуске вас встретит короткое интерактивное обучение.")
                .size(15.5)
                .color(p.on_surface_variant),
        );
        ui.add_space(14.0);
        let dir = PathBuf::from(self.dir.trim());
        feature_row(ui, &p, self.icon("\u{E8B7}", "•"), &format!("Папка: {}", dir.display()), 1.0);
        if self.start_menu {
            feature_row(ui, &p, self.icon("\u{E8FC}", "•"), "Ярлык в меню «Пуск»", 1.0);
        }
        if self.desktop {
            feature_row(ui, &p, self.icon("\u{E7F4}", "•"), "Ярлык на рабочем столе", 1.0);
        }
        ui.add_space(8.0);
        switch_row(ui, &p, "Запустить Notepad You", &mut self.launch);

        let mut bar = bottom_bar(ui, content);
        if pill(&mut bar, self.icon("\u{E73E}", ""), "Готово", p.primary, p.on_primary, 48.0, 24.0, 150.0).clicked() {
            if self.launch {
                let _ = Command::new(dir.join("NotepadYou.exe")).current_dir(&dir).spawn();
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn page_error(&mut self, ui: &mut Ui, content: Rect, ctx: &egui::Context) {
        let p = self.pal;
        ui.label(RichText::new("Что-то пошло не так").font(FontId::new(30.0, bold_family())).color(p.on_surface));
        ui.add_space(6.0);
        Frame::none()
            .fill(p.secondary_container)
            .rounding(16.0)
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(&self.error).size(14.5).color(p.on_secondary_container));
            });
        let mut bar = bottom_bar(ui, content);
        if pill(&mut bar, "", "Закрыть", p.primary, p.on_primary, 48.0, 24.0, 130.0).clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if pill(&mut bar, "", "Попробовать снова", Color32::TRANSPARENT, p.primary, 48.0, 24.0, 0.0).clicked() {
            let back = if self.uninstall { Page::UninstallConfirm } else { Page::Welcome };
            self.go(back, -1.0);
        }
    }

    fn page_uninstall_confirm(&mut self, ui: &mut Ui, content: Rect, ctx: &egui::Context) {
        let p = self.pal;
        let el = self.page_since.elapsed().as_secs_f32();
        ui.label(RichText::new("Удалить Notepad You?").font(FontId::new(30.0, bold_family())).color(p.on_surface));
        ui.label(RichText::new("Ваши заметки (.md файлы) останутся на месте.").size(15.0).color(p.on_surface_variant));
        ui.add_space(6.0);
        let dir = self.dir.trim().to_string();
        let items = [
            (self.icon("\u{E8B7}", "•"), format!("Программа: {}", short_path(&dir, 30))),
            (self.icon("\u{E8FC}", "•"), "Ярлыки в «Пуске» и на рабочем столе".to_string()),
            (self.icon("\u{E8E5}", "•"), "Пункт «Открыть с помощью» для .md и .txt".to_string()),
            (self.icon("\u{E713}", "•"), "Запись в «Установленных приложениях»".to_string()),
        ];
        for (i, (ic, text)) in items.iter().enumerate() {
            let t = ease_out((el - 0.15 - i as f32 * 0.09) / 0.4);
            feature_row(ui, &p, ic, text, t);
        }
        let t = ease_out((el - 0.55) / 0.4);
        ui.scope(|ui| {
            ui.set_opacity(t);
            switch_row(ui, &p, "Удалить также настройки программы", &mut self.remove_settings);
        });

        let mut bar = bottom_bar(ui, content);
        if pill(&mut bar, self.icon("\u{E74D}", ""), "Удалить", p.primary, p.on_primary, 48.0, 24.0, 140.0).clicked() {
            self.start_work();
        }
        if pill(&mut bar, "", "Отмена", p.secondary_container, p.on_secondary_container, 48.0, 24.0, 0.0).clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn page_uninstall_done(&mut self, ui: &mut Ui, content: Rect, ctx: &egui::Context) {
        let p = self.pal;
        let el = self.page_since.elapsed().as_secs_f32();
        let pop = back_out((el / 0.5).clamp(0.0, 1.0));
        ui.label(
            RichText::new("Notepad You удалён")
                .font(FontId::new(22.0 + 8.0 * pop, bold_family()))
                .color(p.on_surface),
        );
        ui.label(
            RichText::new("Спасибо, что пользовались! Ваши заметки остались на месте — будем рады видеть вас снова.")
                .size(15.5)
                .color(p.on_surface_variant),
        );
        ui.add_space(14.0);
        let done = [
            "Программа удалена",
            "Ярлыки убраны",
            "Система очищена",
        ];
        for (i, text) in done.iter().enumerate() {
            let t = ease_out((el - 0.25 - i as f32 * 0.12) / 0.4);
            feature_row(ui, &p, self.icon("\u{E73E}", "✔"), text, t);
        }
        let mut bar = bottom_bar(ui, content);
        if pill(&mut bar, self.icon("\u{E73E}", ""), "Закрыть", p.primary, p.on_primary, 48.0, 24.0, 140.0).clicked() {
            schedule_self_cleanup(Path::new(self.dir.trim()));
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn page_not_installed(&mut self, ui: &mut Ui, content: Rect, ctx: &egui::Context) {
        let p = self.pal;
        chip(ui, &p, &format!("Удаление · версия {VERSION}"));
        ui.add_space(10.0);
        ui.label(RichText::new("Notepad You не найден").font(FontId::new(30.0, bold_family())).color(p.on_surface));
        ui.label(
            RichText::new("Похоже, программа уже удалена или установлена в другую папку. Удалять нечего.")
                .size(15.5)
                .color(p.on_surface_variant),
        );
        let mut bar = bottom_bar(ui, content);
        if pill(&mut bar, "", "Закрыть", p.primary, p.on_primary, 48.0, 24.0, 130.0).clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}

impl eframe::App for Setup {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // прозрачное окно — рисуем свои скруглённые углы
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = ctx.input(|i| i.time) as f32;
        ctx.request_repaint();

        if self.page == Page::Progress {
            let (finished, error) = {
                let s = self.shared.lock().unwrap();
                (s.finished, s.error.clone())
            };
            // Даём анимации прогресса дойти до конца
            if finished && (error.is_some() || self.shown_progress > 0.985) {
                if let Some(e) = error {
                    self.error = e;
                    self.go(Page::Error, 1.0);
                } else if self.uninstall {
                    self.go(Page::UninstallDone, 1.0);
                } else {
                    self.go(Page::Done, 1.0);
                }
            }
        }

        let p = self.pal;
        CentralPanel::default().frame(Frame::none()).show(ctx, |ui| {
            let full = ui.max_rect();
            let painter = ui.painter().clone();
            painter.rect_filled(full, 18.0, p.surface);
            painter.rect_stroke(full.shrink(0.5), 18.0, Stroke::new(1.0_f32, p.outline_variant));

            // Левая панель с анимированной «печенькой»
            let hero = Rect::from_min_size(full.min + vec2(10.0, 10.0), vec2(290.0, full.height() - 20.0));
            let progress = match self.page {
                Page::Progress => Some(self.shown_progress),
                Page::Done | Page::UninstallDone => Some(1.0),
                _ => None,
            };
            let done_t = match self.page {
                Page::Done | Page::UninstallDone => ease_out(self.page_since.elapsed().as_secs_f32() / 0.6),
                _ => 0.0,
            };
            draw_hero(&painter, hero, &p, time, progress, done_t, self.logo.id(), self.uninstall);

            // Перетаскивание окна за верхнюю полосу
            let drag_rect = Rect::from_min_max(full.min, pos2(full.right() - 110.0, full.top() + 46.0));
            let drag = ui.interact(drag_rect, Id::new("drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }

            // Кнопки окна
            let btns = Rect::from_min_max(pos2(full.right() - 110.0, full.top() + 8.0), pos2(full.right() - 12.0, full.top() + 54.0));
            let mut bui = ui.new_child(UiBuilder::new().max_rect(btns).layout(Layout::right_to_left(Align::Center)));
            let busy = self.page == Page::Progress;
            if icon_button(&mut bui, self.icon("\u{E8BB}", "✖"), "Закрыть", &p).clicked() && !busy {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if icon_button(&mut bui, self.icon("\u{E921}", "–"), "Свернуть", &p).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }

            // Содержимое страницы: выезжает сбоку и проявляется
            let t = ease_out(self.page_since.elapsed().as_secs_f32() / 0.42);
            let content = Rect::from_min_max(pos2(hero.right() + 34.0, full.top() + 60.0), pos2(full.right() - 34.0, full.bottom() - 28.0));
            let shifted = content.translate(vec2((1.0 - t) * 56.0 * self.page_dir, 0.0));
            let mut cui = ui.new_child(UiBuilder::new().max_rect(shifted).layout(Layout::top_down(Align::Min)));
            cui.set_opacity(t);
            match self.page {
                Page::Welcome => self.page_welcome(&mut cui, shifted),
                Page::Options => self.page_options(&mut cui, shifted),
                Page::Progress => self.page_progress(&mut cui, time),
                Page::Done => self.page_done(&mut cui, shifted, ctx),
                Page::Error => self.page_error(&mut cui, shifted, ctx),
                Page::UninstallConfirm => self.page_uninstall_confirm(&mut cui, shifted, ctx),
                Page::UninstallDone => self.page_uninstall_done(&mut cui, shifted, ctx),
                Page::NotInstalled => self.page_not_installed(&mut cui, shifted, ctx),
            }
        });
    }
}

// --------------------------------------------------------------------- рисование

fn back_out(t: f32) -> f32 {
    // лёгкий «перелёт» и возврат — эффект пружинки
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

#[allow(clippy::too_many_arguments)]
fn draw_hero(
    painter: &Painter,
    rect: Rect,
    p: &Palette,
    time: f32,
    progress: Option<f32>,
    done_t: f32,
    logo: egui::TextureId,
    uninstall: bool,
) {
    painter.rect_filled(rect, 14.0, p.primary_container);
    let clip = painter.with_clip_rect(rect);

    // Плавающие декоративные печеньки
    let deco = [
        (0.18, 0.16, 34.0, 7, 0.35, 0.7),
        (0.82, 0.22, 22.0, 6, -0.5, 1.1),
        (0.12, 0.80, 26.0, 9, 0.6, 0.9),
        (0.86, 0.74, 40.0, 8, -0.3, 0.6),
        (0.55, 0.92, 16.0, 5, 0.8, 1.3),
    ];
    for (i, (fx, fy, r, lobes, spin, speed)) in deco.iter().enumerate() {
        let ph = time * speed + i as f32;
        let c = pos2(
            rect.left() + rect.width() * fx + ph.sin() * 8.0,
            rect.top() + rect.height() * fy + (ph * 0.8).cos() * 10.0,
        );
        cookie(&clip, c, *r, *lobes, 0.14, time * spin, p.primary.gamma_multiply(0.13));
    }

    let c = pos2(rect.center().x, rect.center().y - 30.0 + (time * 1.3).sin() * 6.0);
    let bounce = if done_t > 0.0 { 1.0 + 0.16 * (done_t * PI).sin() } else { 1.0 };
    let r = 74.0 * bounce;

    // Волнистое кольцо прогресса (Material 3 Expressive)
    if let Some(pr) = progress {
        wavy_arc(&clip, c, 104.0, -PI / 2.0, TAU, 0.0, 0.0, 0.0, Stroke::new(5.0_f32, p.primary.gamma_multiply(0.18)));
        if pr > 0.001 {
            let amp = 3.5 * (1.0 - done_t);
            wavy_arc(&clip, c, 104.0, -PI / 2.0, TAU * pr.min(1.0), amp, 14.0, -time * 5.0, Stroke::new(5.5_f32, p.primary));
        }
    }

    // Сама иконка приложения, медленно покачивается.
    // При удалении она уменьшается и бледнеет по мере прогресса.
    let fade_out = if uninstall { progress.unwrap_or(0.0).clamp(0.0, 1.0) } else { 0.0 };
    let size = r * 2.15 * (1.0 - 0.35 * fade_out);
    let img_rect = Rect::from_center_size(c, vec2(size, size));
    let wobble = (time * 0.9).sin() * 0.12 + fade_out * 0.6;
    let mut mesh = egui::Mesh::with_texture(logo);
    let tint = Color32::WHITE.gamma_multiply(1.0 - 0.65 * fade_out);
    mesh.add_rect_with_uv(img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
    mesh.rotate(egui::emath::Rot2::from_angle(wobble), c);
    clip.add(Shape::mesh(mesh));
    // В конце поверх проявляется фиолетовая печенька с галочкой
    let lilac = Color32::from_rgb(0xAE, 0x93, 0xF5);
    if done_t > 0.0 {
        cookie(&clip, c, r * 1.02, 12, 0.075, wobble, lilac.gamma_multiply(done_t));
    }
    if done_t > 0.0 {
        let s = done_t;
        let pts = [pos2(c.x - 28.0, c.y + 2.0), pos2(c.x - 8.0, c.y + 22.0), pos2(c.x + 30.0, c.y - 18.0)];
        // Рисуем галочку «по частям» по мере анимации
        let l1 = pts[0].distance(pts[1]);
        let l2 = pts[1].distance(pts[2]);
        let len = (l1 + l2) * s;
        let mut path = vec![pts[0]];
        if len <= l1 {
            path.push(pts[0].lerp(pts[1], len / l1));
        } else {
            path.push(pts[1]);
            path.push(pts[1].lerp(pts[2], ((len - l1) / l2).min(1.0)));
        }
        clip.add(Shape::line(path, Stroke::new(11.0_f32, Color32::WHITE)));
    }

    // Подпись: при удалении в конце сменяется на «До встречи!»
    let bye = if uninstall { done_t } else { 0.0 };
    let title_pos = pos2(rect.center().x, rect.bottom() - 74.0);
    if bye < 1.0 {
        clip.text(
            title_pos + vec2(0.0, -10.0 * bye),
            Align2::CENTER_CENTER,
            "Notepad You",
            FontId::new(26.0, bold_family()),
            p.on_primary_container.gamma_multiply(1.0 - bye),
        );
    }
    if bye > 0.0 {
        clip.text(
            title_pos + vec2(0.0, 10.0 * (1.0 - bye)),
            Align2::CENTER_CENTER,
            "До встречи!",
            FontId::new(26.0, bold_family()),
            p.on_primary_container.gamma_multiply(bye),
        );
    }
    clip.text(
        pos2(rect.center().x, rect.bottom() - 46.0),
        Align2::CENTER_CENTER,
        format!("Material You · Markdown · {VERSION}"),
        FontId::proportional(13.0),
        p.on_primary_container.gamma_multiply(0.75),
    );
}

#[allow(clippy::too_many_arguments)]
fn wavy_arc(painter: &Painter, c: egui::Pos2, r: f32, from: f32, sweep: f32, amp: f32, waves: f32, phase: f32, stroke: Stroke) {
    let n = ((sweep.abs() * r) / 2.5).max(3.0) as usize;
    let pts: Vec<egui::Pos2> = (0..=n)
        .map(|i| {
            let a = from + sweep * i as f32 / n as f32;
            let rr = r + amp * (waves * a + phase).sin();
            c + vec2(a.cos(), a.sin()) * rr
        })
        .collect();
    painter.add(Shape::line(pts, stroke));
}

/// Линейный волнистый индикатор прогресса.
fn wavy_bar(painter: &Painter, rect: Rect, progress: f32, time: f32, p: &Palette) {
    let y = rect.center().y;
    let x0 = rect.left() + 3.0;
    let x1 = rect.right() - 3.0;
    let xe = x0 + (x1 - x0) * progress.clamp(0.0, 1.0);
    if xe + 8.0 < x1 {
        painter.line_segment([pos2(xe + 8.0, y), pos2(x1, y)], Stroke::new(5.0_f32, p.primary.gamma_multiply(0.18)));
        painter.circle_filled(pos2(x1, y), 2.5, p.primary);
    }
    if xe > x0 + 1.0 {
        let n = ((xe - x0) / 2.0) as usize + 2;
        let pts: Vec<egui::Pos2> = (0..=n)
            .map(|i| {
                let x = x0 + (xe - x0) * i as f32 / n as f32;
                pos2(x, y + 4.0 * ((x - x0) / 9.0 - time * 6.0).sin())
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(5.0_f32, p.primary)));
    }
}

/// Сокращает длинный путь: «C:\Users\…\Programs\Notepad You».
fn short_path(path: &str, max: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max {
        return path.to_string();
    }
    let tail: String = chars[chars.len() - (max - 4)..].iter().collect();
    let head: String = chars.iter().take(3).collect();
    format!("{head}…{tail}")
}

fn chip(ui: &mut Ui, p: &Palette, text: &str) {
    Frame::none()
        .fill(p.secondary_container)
        .rounding(10.0)
        .inner_margin(Margin::symmetric(12.0, 5.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.5).color(p.on_secondary_container));
        });
}

fn feature_row(ui: &mut Ui, p: &Palette, icon: &str, text: &str, t: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    if t <= 0.0 {
        return;
    }
    let off = (1.0 - t) * 24.0;
    let painter = ui.painter();
    let c = pos2(rect.left() + 18.0 + off, rect.center().y);
    cookie(painter, c, 17.0, 8, 0.12, t * 0.8, p.secondary_container.gamma_multiply(t));
    painter.text(c, Align2::CENTER_CENTER, icon, icon_font(14.0), p.on_secondary_container.gamma_multiply(t));
    painter.text(
        pos2(rect.left() + 48.0 + off, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(15.0),
        p.on_surface.gamma_multiply(t),
    );
}

fn step_row(ui: &mut Ui, p: &Palette, text: &str, state: u8, time: f32, check: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
    let painter = ui.painter();
    let c = pos2(rect.left() + 16.0, rect.center().y);
    match state {
        2 => {
            cookie(painter, c, 13.0, 8, 0.12, 0.0, p.primary);
            painter.text(c, Align2::CENTER_CENTER, check, icon_font(11.0), p.on_primary);
        }
        1 => cookie(painter, c, 12.0, 7, 0.16, time * 2.5, p.primary.gamma_multiply(0.75)),
        _ => {
            painter.circle_stroke(c, 10.0, Stroke::new(2.0_f32, p.outline_variant));
        }
    }
    let color = if state == 0 { p.on_surface_variant } else { p.on_surface };
    painter.text(pos2(rect.left() + 42.0, rect.center().y), Align2::LEFT_CENTER, text, FontId::proportional(15.0), color);
}

fn switch_row(ui: &mut Ui, p: &Palette, label: &str, value: &mut bool) {
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 42.0), Layout::left_to_right(Align::Center), |ui| {
        ui.label(RichText::new(label).size(15.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            switch(ui, value, p);
        });
    });
}

/// Нижняя полоса с кнопками (раскладка справа налево).
fn bottom_bar(ui: &mut Ui, content: Rect) -> Ui {
    let bar = Rect::from_min_max(pos2(content.left(), content.bottom() - 50.0), content.max);
    ui.new_child(UiBuilder::new().max_rect(bar).layout(Layout::right_to_left(Align::Center)))
}

// --------------------------------------------------------------------- система

fn default_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("C:\\"));
    base.join("Programs").join("Notepad You")
}

fn installed_dir() -> Option<PathBuf> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(UNINSTALL_KEY).ok()?;
    let loc: String = key.get_value("InstallLocation").ok()?;
    Some(PathBuf::from(loc))
}

fn system_dark() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        .map(|v| v == 0)
        .unwrap_or(false)
}

fn set_status(sh: &Arc<Mutex<Shared>>, progress: f32, status: &str) {
    let mut s = sh.lock().unwrap();
    s.progress = progress;
    s.status = status.to_string();
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn powershell(script: &str) -> bool {
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(event: i32, flags: u32, item1: *const std::ffi::c_void, item2: *const std::ffi::c_void);
}

fn notify_shell() {
    // SHCNE_ASSOCCHANGED — обновить значки и ассоциации
    unsafe { SHChangeNotify(0x0800_0000, 0, std::ptr::null(), std::ptr::null()) }
}

fn do_install(o: &Opts, sh: &Arc<Mutex<Shared>>) -> Result<(), String> {
    use std::io::Write;
    set_status(sh, 0.04, "Подготовка…");
    std::thread::sleep(Duration::from_millis(350));
    std::fs::create_dir_all(&o.dir).map_err(|e| format!("Не удалось создать папку «{}»: {e}", o.dir.display()))?;

    let exe = o.dir.join("NotepadYou.exe");
    set_status(sh, 0.08, "Копирование файлов…");
    let mut f = std::fs::File::create(&exe).map_err(|e| {
        format!("Не удалось записать {}.\nЕсли Notepad You сейчас открыт — закройте его и попробуйте снова.\n\n({e})", exe.display())
    })?;
    let chunks: Vec<&[u8]> = APP_EXE.chunks(192 * 1024).collect();
    for (i, c) in chunks.iter().enumerate() {
        f.write_all(c).map_err(|e| format!("Ошибка записи файла: {e}"))?;
        set_status(sh, 0.08 + 0.5 * (i + 1) as f32 / chunks.len() as f32, "Копирование файлов…");
        std::thread::sleep(Duration::from_millis(18));
    }
    drop(f);

    // Копия установщика — для удаления через «Установленные приложения»
    let uninst = o.dir.join("Uninstall.exe");
    if let Ok(me) = std::env::current_exe() {
        if me != uninst {
            let _ = std::fs::copy(&me, &uninst);
        }
    }

    set_status(sh, 0.64, "Создание ярлыков…");
    let exe_s = exe.display().to_string();
    let dir_s = o.dir.display().to_string();
    let lnk = |folder: &str| {
        format!(
            "$s = New-Object -ComObject WScript.Shell; \
             $l = $s.CreateShortcut((Join-Path ([Environment]::GetFolderPath('{folder}')) 'Notepad You.lnk')); \
             $l.TargetPath = {exe}; $l.WorkingDirectory = {dir}; $l.IconLocation = {icon}; \
             $l.Description = 'Notepad You — блокнот в стиле Material You'; $l.Save()",
            exe = ps_quote(&exe_s),
            dir = ps_quote(&dir_s),
            icon = ps_quote(&format!("{exe_s},0")),
        )
    };
    let mut script = String::new();
    if o.start_menu {
        script.push_str(&lnk("Programs"));
        script.push_str("; ");
        // Ярлык удаления рядом с программой в «Пуске»
        let un = uninst.display().to_string();
        script.push_str(&format!(
            "$u = $s.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'Удалить Notepad You.lnk')); \
             $u.TargetPath = {t}; $u.Arguments = '--uninstall'; $u.WorkingDirectory = {d}; $u.IconLocation = {i}; \
             $u.Description = 'Удалить Notepad You'; $u.Save(); ",
            t = ps_quote(&un),
            d = ps_quote(&dir_s),
            i = ps_quote(&format!("{un},0")),
        ));
    }
    if o.desktop {
        script.push_str(&lnk("Desktop"));
    }
    if !script.is_empty() {
        powershell(&script);
    }
    set_status(sh, 0.8, "Создание ярлыков…");
    std::thread::sleep(Duration::from_millis(250));

    set_status(sh, 0.84, "Регистрация в системе…");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let reg_err = |e: std::io::Error| format!("Ошибка записи в реестр: {e}");
    let (k, _) = hkcu.create_subkey(UNINSTALL_KEY).map_err(reg_err)?;
    k.set_value("DisplayName", &"Notepad You").map_err(reg_err)?;
    k.set_value("DisplayVersion", &VERSION).map_err(reg_err)?;
    k.set_value("Publisher", &"Notepad You").map_err(reg_err)?;
    k.set_value("DisplayIcon", &format!("\"{exe_s}\",0")).map_err(reg_err)?;
    k.set_value("InstallLocation", &dir_s).map_err(reg_err)?;
    k.set_value("UninstallString", &format!("\"{}\" --uninstall", uninst.display())).map_err(reg_err)?;
    k.set_value("NoModify", &1u32).map_err(reg_err)?;
    k.set_value("NoRepair", &1u32).map_err(reg_err)?;
    k.set_value("EstimatedSize", &((APP_EXE.len() as u32) * 2 / 1024)).map_err(reg_err)?;

    if o.assoc {
        let open_cmd = format!("\"{exe_s}\" \"%1\"");
        let (pk, _) = hkcu.create_subkey(r"Software\Classes\NotepadYou.md").map_err(reg_err)?;
        pk.set_value("", &"Документ Markdown").map_err(reg_err)?;
        let (ik, _) = hkcu.create_subkey(r"Software\Classes\NotepadYou.md\DefaultIcon").map_err(reg_err)?;
        ik.set_value("", &format!("\"{exe_s}\",0")).map_err(reg_err)?;
        let (ck, _) = hkcu.create_subkey(r"Software\Classes\NotepadYou.md\shell\open\command").map_err(reg_err)?;
        ck.set_value("", &open_cmd).map_err(reg_err)?;
        let (ak, _) = hkcu.create_subkey(r"Software\Classes\Applications\NotepadYou.exe").map_err(reg_err)?;
        ak.set_value("FriendlyAppName", &"Notepad You").map_err(reg_err)?;
        let (ac, _) = hkcu
            .create_subkey(r"Software\Classes\Applications\NotepadYou.exe\shell\open\command")
            .map_err(reg_err)?;
        ac.set_value("", &open_cmd).map_err(reg_err)?;
        let (st, _) = hkcu
            .create_subkey(r"Software\Classes\Applications\NotepadYou.exe\SupportedTypes")
            .map_err(reg_err)?;
        for ext in [".md", ".markdown", ".txt"] {
            st.set_value(ext, &"").map_err(reg_err)?;
            let (ok, _) = hkcu
                .create_subkey(format!(r"Software\Classes\{ext}\OpenWithProgids"))
                .map_err(reg_err)?;
            ok.set_value("NotepadYou.md", &"").map_err(reg_err)?;
        }
    }
    notify_shell();
    std::thread::sleep(Duration::from_millis(300));
    set_status(sh, 1.0, "Готово!");
    Ok(())
}

fn do_uninstall(o: &Opts, sh: &Arc<Mutex<Shared>>) -> Result<(), String> {
    set_status(sh, 0.08, "Удаление ярлыков…");
    std::thread::sleep(Duration::from_millis(300));
    powershell(
        "foreach ($f in 'Programs','Desktop') { foreach ($n in 'Notepad You.lnk','Удалить Notepad You.lnk') { \
           Remove-Item -LiteralPath (Join-Path ([Environment]::GetFolderPath($f)) $n) -ErrorAction SilentlyContinue } }",
    );
    set_status(sh, 0.35, "Очистка реестра…");
    std::thread::sleep(Duration::from_millis(250));
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);
    let _ = hkcu.delete_subkey_all(r"Software\Classes\NotepadYou.md");
    let _ = hkcu.delete_subkey_all(r"Software\Classes\Applications\NotepadYou.exe");
    for ext in [".md", ".markdown", ".txt"] {
        if let Ok(k) = hkcu.open_subkey_with_flags(format!(r"Software\Classes\{ext}\OpenWithProgids"), KEY_SET_VALUE) {
            let _ = k.delete_value("NotepadYou.md");
        }
    }
    notify_shell();
    set_status(sh, 0.6, "Удаление файлов…");
    std::thread::sleep(Duration::from_millis(250));
    let exe = o.dir.join("NotepadYou.exe");
    if exe.exists() {
        std::fs::remove_file(&exe).map_err(|e| {
            format!("Не удалось удалить {}.\nЕсли Notepad You открыт — закройте его и попробуйте снова.\n\n({e})", exe.display())
        })?;
    }
    if o.remove_settings {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let _ = std::fs::remove_dir_all(PathBuf::from(appdata).join("Notepad You"));
        }
    }
    set_status(sh, 1.0, "Готово");
    Ok(())
}

/// Удаляет Uninstall.exe и папку (только если она пуста) после выхода из программы.
fn schedule_self_cleanup(dir: &Path) {
    let uninst = dir.join("Uninstall.exe");
    let cmd = format!(
        "ping 127.0.0.1 -n 3 > nul & del /f /q \"{}\" & rmdir \"{}\"",
        uninst.display(),
        dir.display()
    );
    // raw_arg: cmd.exe не понимает экранирование кавычек, которое делает Rust
    let _ = Command::new("cmd").raw_arg(format!("/C \"{cmd}\"")).creation_flags(NO_WINDOW).spawn();
}
