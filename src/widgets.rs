//! Виджеты в стиле Material 3: круглые иконки-кнопки, «таблетки», переключатель, сегменты.

use crate::theme::{bold_family, icon_family, mix, Palette};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, FontFamily, FontId, Mesh, Pos2, Rect, Response,
    Rounding, Sense, Shape, Stroke, Ui,
};
use std::f32::consts::TAU;

pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, icon_family())
}

/// Набор глифов: Segoe MDL2 / Fluent Icons или запасные Unicode-символы.
#[derive(Clone, Copy)]
pub struct Icons {
    pub add: &'static str,
    pub open: &'static str,
    pub save: &'static str,
    pub save_as: &'static str,
    pub menu: &'static str,
    pub check: &'static str,
    pub close: &'static str,
    pub sun: &'static str,
    pub moon: &'static str,
    pub settings: &'static str,
    pub bold: &'static str,
    pub italic: &'static str,
    pub code: &'static str,
    pub link: &'static str,
    pub list: &'static str,
    pub doc: &'static str,
}

pub const MDL2: Icons = Icons {
    add: "\u{E710}",
    open: "\u{E8E5}",
    save: "\u{E74E}",
    save_as: "\u{E792}",
    menu: "\u{E700}",
    check: "\u{E73E}",
    close: "\u{E711}",
    sun: "\u{E706}",
    moon: "\u{E708}",
    settings: "\u{E713}",
    bold: "\u{E8DD}",
    italic: "\u{E8DB}",
    code: "\u{E943}",
    link: "\u{E71B}",
    list: "\u{E8FD}",
    doc: "\u{E8A5}",
};

pub const FALLBACK: Icons = Icons {
    add: "➕",
    open: "🗁",
    save: "💾",
    save_as: "💾",
    menu: "☰",
    check: "✔",
    close: "✖",
    sun: "☀",
    moon: "🌙",
    settings: "⚙",
    bold: "B",
    italic: "I",
    code: "</>",
    link: "🔗",
    list: "☰",
    doc: "📄",
};

/// Фигура «печенька» (Cookie из Material 3 Expressive): круг с волнистым краем.
/// `lobes` — число «зубчиков», `depth` — глубина волны (0..1), `rot` — поворот в радианах.
pub fn cookie(painter: &egui::Painter, center: Pos2, radius: f32, lobes: u32, depth: f32, rot: f32, color: Color32) {
    if color.a() == 0 || radius <= 0.5 {
        return;
    }
    let seg = (lobes as usize * 12).max(64);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..seg {
        let a = i as f32 / seg as f32 * TAU;
        let wave = 0.5 + 0.5 * (lobes as f32 * (a - rot)).cos();
        let r = radius * (1.0 - depth + depth * wave);
        let dir = vec2(a.cos(), a.sin());
        // внутренняя точка + внешняя прозрачная — сглаживание края
        mesh.colored_vertex(center + dir * (r - 0.5), color);
        mesh.colored_vertex(center + dir * (r + 0.6), Color32::TRANSPARENT);
    }
    for i in 0..seg as u32 {
        let j = (i + 1) % seg as u32;
        let (ii, io, ji, jo) = (1 + 2 * i, 2 + 2 * i, 1 + 2 * j, 2 + 2 * j);
        mesh.add_triangle(0, ii, ji);
        mesh.add_triangle(ii, io, jo);
        mesh.add_triangle(ii, jo, ji);
    }
    painter.add(Shape::mesh(mesh));
}

/// Кнопка-иконка в форме печеньки: при наведении увеличивается,
/// окрашивается в акцентный цвет и проворачивается.
pub fn icon_button(ui: &mut Ui, icon: &str, tip: &str, p: &Palette) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let ctx = ui.ctx();
        let h = ease_out(ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.25));
        let down = ctx.animate_bool_with_time(resp.id.with("d"), resp.is_pointer_button_down_on(), 0.08);
        let scale = 1.0 + 0.06 * h - 0.10 * down;
        let rot = h * TAU / 9.0 * 0.5;
        let fill = mix(p.surface_container_high, p.primary_container, h);
        cookie(ui.painter(), rect.center(), 20.0 * scale, 9, 0.09, rot, fill);
        let base = mix(p.on_surface_variant, p.on_primary_container, h);
        let color = if ui.is_enabled() { base } else { base.gamma_multiply(0.4) };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, icon, icon_font(16.0 * scale), color);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tip)
}

/// Кнопка панели форматирования: при наведении под ней «вырастает» печенька.
pub fn tool_button(ui: &mut Ui, label: &str, font: FontId, tip: &str, p: &Palette) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let ctx = ui.ctx();
        let t = ease_out(ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.22));
        let down = ctx.animate_bool_with_time(resp.id.with("d"), resp.is_pointer_button_down_on(), 0.08);
        if t > 0.0 {
            let r = 17.0 * (0.55 + 0.45 * t) * (1.0 - 0.1 * down);
            cookie(ui.painter(), rect.center(), r, 6, 0.12, t * 0.5, p.primary.gamma_multiply(0.16 * t));
        }
        let fg = mix(p.on_surface_variant, p.primary, t);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, fg);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tip)
}

pub fn vertical_divider(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, 24.0), Sense::hover());
    ui.painter().vline(rect.center().x, rect.y_range(), Stroke::new(1.0_f32, p.outline_variant));
}

/// Кнопка-«таблетка» (filled / tonal / text button, FAB).
#[allow(clippy::too_many_arguments)]
pub fn pill(
    ui: &mut Ui,
    icon: &str,
    text: &str,
    fill: Color32,
    fg: Color32,
    height: f32,
    rounding: f32,
    min_width: f32,
) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::new(14.5, bold_family()), fg);
    let icon_w = if icon.is_empty() { 0.0 } else { 26.0 };
    let pad = if height >= 50.0 { 20.0 } else { 18.0 };
    let w = (galley.size().x + icon_w + pad * 2.0).max(min_width);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(resp.id, resp.hovered());
        let painter = ui.painter();
        painter.rect_filled(rect, rounding, fill);
        if t > 0.0 {
            painter.rect_filled(rect, rounding, fg.gamma_multiply(0.08 * t));
        }
        let content_w = galley.size().x + icon_w;
        let mut x = rect.center().x - content_w / 2.0;
        if !icon.is_empty() {
            let c = pos2(x + 9.0, rect.center().y);
            if height >= 50.0 {
                // У FAB иконка «+» сидит на крутящейся печеньке
                let spin = ease_out(ui.ctx().animate_bool_with_time(resp.id.with("spin"), resp.hovered(), 0.35));
                cookie(painter, c, 15.0 + 1.5 * spin, 8, 0.12, spin * TAU / 8.0, fg.gamma_multiply(0.12));
            }
            painter.text(c, Align2::CENTER_CENTER, icon, icon_font(16.0), fg);
            x += icon_w;
        }
        let y = rect.center().y - galley.size().y / 2.0;
        painter.galley(pos2(x, y), galley, fg);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Переключатель Material 3.
pub fn switch(ui: &mut Ui, on: &mut bool, p: &Palette) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(52.0, 32.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ease_out(ui.ctx().animate_bool_with_time(resp.id, *on, 0.25));
        let painter = ui.painter();
        painter.rect_filled(rect, 16.0, mix(p.surface_container_highest, p.primary, t));
        if t < 1.0 {
            painter.rect_stroke(rect.shrink(1.0), 15.0, Stroke::new(2.0_f32, p.outline.gamma_multiply(1.0 - t)));
        }
        let r = egui::lerp(8.0..=12.0, t) + if resp.hovered() { 1.0 } else { 0.0 };
        let x = egui::lerp((rect.left() + 16.0)..=(rect.right() - 16.0), t);
        painter.circle_filled(pos2(x, rect.center().y), r, mix(p.outline, p.on_primary, t));
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Печенька выбора цвета темы. Выбранная обводится и поворачивается.
pub fn color_dot(ui: &mut Ui, color: Color32, selected: bool, check: &str, p: &Palette) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let ctx = ui.ctx();
        let s = ease_out(ctx.animate_bool_with_time(resp.id.with("s"), selected, 0.3));
        let h = ease_out(ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.2));
        let painter = ui.painter();
        let c = rect.center();
        let rot = s * TAU / 7.0 * 0.5;
        if s > 0.0 {
            cookie(painter, c, 21.0, 7, 0.10, rot, p.on_surface.gamma_multiply(s));
            cookie(painter, c, 18.5, 7, 0.10, rot, p.surface_container_high);
        }
        let r = 15.5 + 1.5 * h - 1.5 * s;
        cookie(painter, c, r, 7, 0.10, rot, color);
        if s > 0.01 {
            let fg = if color.r() as u32 + color.g() as u32 + color.b() as u32 > 420 {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            painter.text(c, Align2::CENTER_CENTER, check, icon_font(13.0 * s), fg.gamma_multiply(s));
        }
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Сегментированная кнопка (выбор режима просмотра).
/// Учитывает раскладку справа-налево, сохраняя визуальный порядок.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)], check: &str, p: &Palette) {
    ui.spacing_mut().item_spacing.x = 0.0;
    let n = options.len();
    let order: Vec<usize> = if ui.layout().prefer_right_to_left() {
        (0..n).rev().collect()
    } else {
        (0..n).collect()
    };
    let font = FontId::new(14.0, FontFamily::Proportional);
    for i in order {
        let (val, label) = options[i];
        let selected = *value == val;
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), p.on_surface);
        let w = galley.size().x + 30.0 + 22.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
        if resp.clicked() {
            *value = val;
        }
        if ui.is_rect_visible(rect) {
            // Плавное появление выделения и галочки
            let t = ease_out(ui.ctx().animate_bool_with_time(resp.id.with("sel"), selected, 0.22));
            let h = ui.ctx().animate_bool_with_time(resp.id.with("hov"), resp.hovered(), 0.12);
            let r = 18.0;
            let left = if i == 0 { r } else { 0.0 };
            let right = if i == n - 1 { r } else { 0.0 };
            let rounding = Rounding { nw: left, sw: left, ne: right, se: right };
            let painter = ui.painter();
            let fg = mix(p.on_surface, p.on_secondary_container, t);
            if t > 0.0 {
                painter.rect_filled(rect, rounding, p.secondary_container.gamma_multiply(t));
            }
            if h > 0.0 {
                painter.rect_filled(rect, rounding, fg.gamma_multiply(0.08 * h));
            }
            painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, p.outline));
            let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), fg);
            let content = galley.size().x + 22.0 * t;
            let mut x = rect.center().x - content / 2.0;
            if t > 0.01 {
                let c = pos2(x + 7.0, rect.center().y);
                painter.text(c, Align2::CENTER_CENTER, check, icon_font(13.0 * (0.4 + 0.6 * t)), fg.gamma_multiply(t));
                x += 22.0 * t;
            }
            painter.galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, fg);
        }
        resp.on_hover_cursor(CursorIcon::PointingHand);
    }
}

/// Плавное замедление в конце (Material «emphasized decelerate»-подобное).
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn centered_rect(center: egui::Pos2, size: f32) -> Rect {
    Rect::from_center_size(center, vec2(size, size))
}
