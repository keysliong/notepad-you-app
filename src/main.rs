// Скрываем консольное окно в Windows
#![windows_subsystem = "windows"]

mod app;
mod demo;
mod emoji;
mod highlight;
mod icon;
mod theme;
mod widgets;

use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // Файл, переданный в командной строке («Открыть с помощью…»)
    let file = std::env::args_os().nth(1).map(PathBuf::from);

    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Notepad You")
            .with_app_id("notepad-you")
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([640.0, 420.0])
            .with_drag_and_drop(true)
            // Пустая иконка отключает встроенную установку значка в eframe:
            // значок окна в цветах темы ставит само приложение (app.rs, set_window_icon).
            // В Проводнике остаётся фиолетовая иконка из ресурсов .exe.
            .with_icon(egui::IconData::default()),
        ..Default::default()
    };
    // Демо-режим (запись гифок для README) не трогает настоящие настройки пользователя
    if std::env::var_os("NPY_DEMO").is_some() {
        options.persistence_path = Some(std::env::temp_dir().join("notepad-you-demo"));
        options.persist_window = false;
    }

    eframe::run_native(
        "Notepad You",
        options,
        Box::new(move |cc| Ok(Box::new(app::NotepadApp::new(cc, file)))),
    )
}
