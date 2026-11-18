//! Цветовая система в духе Material You: тональная палитра из «цвета-семени».

use eframe::egui::{
    self, epaint::Shadow, vec2, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin,
    Rounding, Stroke, TextStyle,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Seed {
    Coral,
    Lavender,
    Ocean,
    Mint,
    Rose,
    Amber,
    Graphite,
}

impl Seed {
    pub const ALL: [Seed; 7] = [
        Seed::Coral,
        Seed::Lavender,
        Seed::Ocean,
        Seed::Mint,
        Seed::Rose,
        Seed::Amber,
        Seed::Graphite,
    ];

    fn hue_sat(self) -> (f32, f32) {
        match self {
            Seed::Coral => (12.0, 0.78),
            Seed::Lavender => (262.0, 0.42),
            Seed::Ocean => (208.0, 0.55),
            Seed::Mint => (158.0, 0.42),
            Seed::Rose => (340.0, 0.50),
            Seed::Amber => (30.0, 0.62),
            Seed::Graphite => (220.0, 0.10),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Seed::Coral => "Коралл",
            Seed::Lavender => "Лаванда",
            Seed::Ocean => "Океан",
            Seed::Mint => "Мята",
            Seed::Rose => "Роза",
            Seed::Amber => "Янтарь",
            Seed::Graphite => "Графит",
        }
    }
}

/// Цветовые роли Material 3.
#[derive(Clone, Copy)]
pub struct Palette {
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub secondary_container: Color32,
    pub on_secondary_container: Color32,
    pub tertiary: Color32,
    pub surface: Color32,
    pub surface_container_low: Color32,
    pub surface_container: Color32,
    pub surface_container_high: Color32,
    pub surface_container_highest: Color32,
    pub on_surface: Color32,
    pub on_surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
    pub inverse_surface: Color32,
    pub inverse_on_surface: Color32,
}

impl Palette {
    pub fn new(seed: Seed, dark: bool) -> Self {
        let (h, s) = seed.hue_sat();
        let p = |t: f32| tone(h, s, t);
        let sec = |t: f32| tone(h, s * 0.35, t);
        let ter = |t: f32| tone((h + 60.0) % 360.0, s * 0.6, t);
        let n = |t: f32| tone(h, s.min(0.08), t);
        let nv = |t: f32| tone(h, (s * 0.25).min(0.14), t);
        if dark {
            Palette {
                primary: p(80.0),
                on_primary: p(20.0),
                primary_container: p(30.0),
                on_primary_container: p(90.0),
                secondary_container: sec(30.0),
                on_secondary_container: sec(90.0),
                tertiary: ter(80.0),
                surface: n(6.0),
                surface_container_low: n(10.0),
                surface_container: n(12.0),
                surface_container_high: n(17.0),
                surface_container_highest: n(22.0),
                on_surface: n(90.0),
                on_surface_variant: nv(80.0),
                outline: nv(60.0),
                outline_variant: nv(30.0),
                inverse_surface: n(90.0),
                inverse_on_surface: n(20.0),
            }
        } else {
            Palette {
                primary: p(40.0),
                on_primary: p(100.0),
                primary_container: p(90.0),
                on_primary_container: p(10.0),
                secondary_container: sec(90.0),
                on_secondary_container: sec(10.0),
                tertiary: ter(40.0),
                surface: n(98.0),
                surface_container_low: n(96.0),
                surface_container: n(94.0),
                surface_container_high: n(92.0),
                surface_container_highest: n(89.0),
                on_surface: n(10.0),
                on_surface_variant: nv(30.0),
                outline: nv(50.0),
                outline_variant: nv(80.0),
                inverse_surface: n(20.0),
                inverse_on_surface: n(95.0),
            }
        }
    }
}

impl Palette {
    /// Плавный переход между палитрами (для анимации смены темы).
    pub fn lerp(a: &Palette, b: &Palette, t: f32) -> Palette {
        macro_rules! l {
            ($($f:ident),*) => { Palette { $($f: mix(a.$f, b.$f, t)),* } };
        }
        l!(
            primary, on_primary, primary_container, on_primary_container, secondary_container,
            on_secondary_container, tertiary, surface, surface_container_low, surface_container,
            surface_container_high, surface_container_highest, on_surface, on_surface_variant,
            outline, outline_variant, inverse_surface, inverse_on_surface
        )
    }
}

/// Цвета иконки под выбранную тему. «Лаванда» — фирменная фиолетовая иконка.
pub fn icon_colors(seed: Seed) -> crate::icon::IconColors {
    if seed == Seed::Lavender {
        return crate::icon::PURPLE;
    }
    let (h, s) = seed.hue_sat();
    let f = |c: Color32, a: f32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, a];
    let sat = |k: f32| (s * k).min(1.0);
    crate::icon::IconColors {
        cookie: f(hsl(h, sat(1.35), 0.80), 1.0),
        rim: f(hsl(h, sat(1.25), 0.72), 1.0),
        shadow: f(hsl(h, sat(1.0), 0.20), 0.30),
        sheet: f(hsl(h, 0.6, 0.99), 1.0),
        lines: f(hsl(h, sat(1.1), 0.55), 1.0),
        // стикер — соседний оттенок, как розовый у фиолетовой иконки
        sticker: f(hsl((h + 68.0) % 360.0, if s < 0.2 { 0.25 } else { 0.9 }, 0.82), 1.0),
    }
}

fn tone(h: f32, s: f32, t: f32) -> Color32 {
    hsl(h, s, (t / 100.0).clamp(0.0, 1.0))
}

fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h % 360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let f = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(r), f(g), f(b))
}

/// Смешивание двух цветов (для состояний наведения/нажатия).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn icon_family() -> FontFamily {
    FontFamily::Name("icons".into())
}

/// Применяем палитру к стилю egui.
pub fn apply(ctx: &egui::Context, p: &Palette, dark: bool) {
    let mut style = (*ctx.style()).clone();
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };

    v.override_text_color = Some(p.on_surface);
    v.panel_fill = p.surface;
    v.window_fill = p.surface_container_high;
    v.faint_bg_color = p.surface_container;
    v.extreme_bg_color = p.surface_container_highest;
    v.code_bg_color = p.surface_container_highest;
    v.window_stroke = Stroke::NONE;
    v.window_rounding = Rounding::same(28.0);
    v.menu_rounding = Rounding::same(16.0);
    v.window_shadow = Shadow {
        offset: vec2(0.0, 8.0),
        blur: 28.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark { 140 } else { 45 }),
    };
    v.popup_shadow = Shadow {
        offset: vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark { 120 } else { 35 }),
    };
    v.hyperlink_color = p.primary;
    v.selection.bg_fill = mix(p.surface, p.primary, 0.35);
    v.selection.stroke = Stroke::new(1.5_f32, p.primary);
    v.text_cursor.stroke = Stroke::new(2.0_f32, p.primary);
    v.slider_trailing_fill = true;

    let pill = Rounding::same(20.0);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface_container;
    w.noninteractive.weak_bg_fill = p.surface_container;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.outline_variant);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.on_surface_variant);
    w.noninteractive.rounding = Rounding::same(12.0);

    w.inactive.bg_fill = p.surface_container_highest;
    w.inactive.weak_bg_fill = p.surface_container_high;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.on_surface);
    w.inactive.rounding = pill;
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = mix(p.surface_container_highest, p.primary, 0.10);
    w.hovered.weak_bg_fill = mix(p.surface_container_high, p.primary, 0.08);
    w.hovered.bg_stroke = Stroke::NONE;
    w.hovered.fg_stroke = Stroke::new(1.5_f32, p.on_surface);
    w.hovered.rounding = pill;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = mix(p.surface_container_highest, p.primary, 0.18);
    w.active.weak_bg_fill = mix(p.surface_container_high, p.primary, 0.16);
    w.active.bg_stroke = Stroke::NONE;
    w.active.fg_stroke = Stroke::new(1.5_f32, p.on_surface);
    w.active.rounding = pill;
    w.active.expansion = 0.0;
    w.open = w.active;

    style.visuals = v;
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(14.0, 7.0);
    style.spacing.interact_size.y = 34.0;
    style.spacing.menu_margin = Margin::same(12.0);
    style.spacing.window_margin = Margin::same(24.0);
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(22.0, bold_family())),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

/// Шрифт интерфейса и текста.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum UiFont {
    Nunito,
    Segoe,
}

impl UiFont {
    pub fn name(self) -> &'static str {
        match self {
            UiFont::Nunito => "Nunito",
            UiFont::Segoe => "Segoe UI",
        }
    }
}

// Встроенные шрифты (SIL Open Font License)
const NUNITO: &[u8] = include_bytes!("../assets/fonts/Nunito-Regular.ttf");
const NUNITO_BOLD: &[u8] = include_bytes!("../assets/fonts/Nunito-Bold.ttf");
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");

/// Подключаем шрифты: встроенные Nunito / JetBrains Mono и системные Windows
/// (Segoe UI, Consolas, Segoe MDL2 Assets) как запасные.
/// Возвращает `true`, если найден шрифт иконок.
pub fn setup_fonts(ctx: &egui::Context, ui_font: UiFont) -> bool {
    let mut fonts = FontDefinitions::default();
    let dir = std::env::var("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows"))
        .join("Fonts");
    let load = |name: &str| std::fs::read(dir.join(name)).ok();

    let default_prop = fonts.families[&FontFamily::Proportional].clone();
    let default_mono = fonts.families[&FontFamily::Monospace].clone();

    fonts.font_data.insert("nunito".into(), FontData::from_static(NUNITO));
    fonts.font_data.insert("nunito-bold".into(), FontData::from_static(NUNITO_BOLD));
    fonts.font_data.insert("jbmono".into(), FontData::from_static(JETBRAINS_MONO));

    let mut proportional: Vec<String> = Vec::new();
    let mut bold: Vec<String> = Vec::new();
    if ui_font == UiFont::Nunito {
        proportional.push("nunito".into());
        bold.push("nunito-bold".into());
    }
    if let Some(d) = load("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), FontData::from_owned(d));
        proportional.push("segoe".into());
    }
    if ui_font == UiFont::Segoe {
        proportional.push("nunito".into());
    }
    proportional.extend(default_prop);
    if let Some(d) = load("seguisym.ttf") {
        fonts.font_data.insert("segoe-sym".into(), FontData::from_owned(d));
        proportional.push("segoe-sym".into());
    }

    if let Some(d) = load("seguisb.ttf").or_else(|| load("segoeuib.ttf")) {
        fonts.font_data.insert("segoe-bold".into(), FontData::from_owned(d));
        bold.push("segoe-bold".into());
    }
    bold.extend(proportional.iter().cloned());

    let mut mono: Vec<String> = vec!["jbmono".into()];
    if let Some(d) = load("consola.ttf") {
        fonts.font_data.insert("mono".into(), FontData::from_owned(d));
        mono.push("mono".into());
    }
    mono.extend(default_mono);
    mono.extend(proportional.iter().cloned());

    let mut icons: Vec<String> = Vec::new();
    let has_icons = if let Some(d) = load("SegoeIcons.ttf").or_else(|| load("segmdl2.ttf")) {
        fonts.font_data.insert("icons".into(), FontData::from_owned(d));
        icons.push("icons".into());
        true
    } else {
        false
    };
    icons.extend(proportional.iter().cloned());

    fonts.families.insert(FontFamily::Proportional, proportional);
    fonts.families.insert(FontFamily::Monospace, mono);
    fonts.families.insert(bold_family(), bold);
    fonts.families.insert(icon_family(), icons);
    ctx.set_fonts(fonts);
    has_icons
}
