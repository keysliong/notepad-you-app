//! Иконка приложения, нарисованная кодом: светло-фиолетовая «печенька» (Material 3 Cookie),
//! на ней белый листок с тремя строчками и розовая печенька-стикер.
//! Используется и для окна, и в build.rs (иконка .exe и файлы в папке icon/).

pub type Rgba = [f32; 4];

/// Цвета частей иконки.
#[derive(Clone, Copy)]
pub struct IconColors {
    pub cookie: Rgba,
    pub rim: Rgba,
    pub shadow: Rgba,
    pub sheet: Rgba,
    pub lines: Rgba,
    pub sticker: Rgba,
}

/// Фирменная светло-фиолетовая иконка (она же в Проводнике).
pub const PURPLE: IconColors = IconColors {
    cookie: rgb(0xC4, 0xAE, 0xFF),
    rim: rgb(0xAE, 0x93, 0xF5),
    shadow: [0.25, 0.14, 0.50, 0.30],
    sheet: rgb(0xFF, 0xFB, 0xFF),
    lines: rgb(0x8B, 0x6F, 0xD6),
    sticker: rgb(0xFF, 0xB4, 0xD6),
};

const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

/// Фиолетовая иконка размера `n`×`n` (RGBA). Используется build.rs и установщиком.
#[allow(dead_code)]
pub fn icon_rgba(n: u32) -> Vec<u8> {
    icon_rgba_with(n, &PURPLE)
}

/// Иконка размера `n`×`n` в заданных цветах (RGBA).
pub fn icon_rgba_with(n: u32, colors: &IconColors) -> Vec<u8> {
    let s = n as f32 / 64.0; // рисуем в координатах 64×64
    let mut out = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            // 4×4 суперсэмплинг для мелких размеров, 2×2 для крупных
            let ss = if n <= 64 { 4 } else { 2 };
            let mut acc = [0.0f32; 4];
            for sy in 0..ss {
                for sx in 0..ss {
                    let fx = (x as f32 + (sx as f32 + 0.5) / ss as f32) / s;
                    let fy = (y as f32 + (sy as f32 + 0.5) / ss as f32) / s;
                    let c = shade(fx, fy, 0.5 / s, colors);
                    // накапливаем в premultiplied-виде
                    acc[0] += c[0] * c[3];
                    acc[1] += c[1] * c[3];
                    acc[2] += c[2] * c[3];
                    acc[3] += c[3];
                }
            }
            let k = (ss * ss) as f32;
            let a = acc[3] / k;
            let (r, g, b) = if a > 0.0 {
                (acc[0] / k / a, acc[1] / k / a, acc[2] / k / a)
            } else {
                (0.0, 0.0, 0.0)
            };
            let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            out.extend_from_slice(&[q(r), q(g), q(b), q(a)]);
        }
    }
    out
}

/// Цвет точки (в координатах 64×64). `aa` — полуширина сглаживания.
fn shade(x: f32, y: f32, aa: f32, c: &IconColors) -> Rgba {
    let mut col: Rgba = [0.0; 4];

    // 1. Печенька: 12 «зубчиков»
    let d_cookie = cookie_sdf(x, y, 32.0, 32.0, 30.5, 12, 0.075, 0.0);
    over(&mut col, c.rim, cov(d_cookie, aa));
    over(&mut col, c.cookie, cov(d_cookie + 2.2, aa));

    // Листок наклонён на −8°
    let (cx, cy, ang) = (31.0, 33.5, -8.0_f32.to_radians());
    let local = |px: f32, py: f32| {
        let (dx, dy) = (px - cx, py - cy);
        let (c, s) = (ang.cos(), ang.sin());
        (dx * c + dy * s, -dx * s + dy * c)
    };

    // 2. Мягкая тень под листком
    let (sx, sy) = local(x - 1.2, y - 2.4);
    let d_shadow = rrect_sdf(sx, sy, 14.5, 17.5, 4.5);
    over(&mut col, c.shadow, cov(d_shadow, aa + 1.2));

    // 3. Листок
    let (lx, ly) = local(x, y);
    let d_sheet = rrect_sdf(lx, ly, 14.0, 17.0, 4.0);
    over(&mut col, c.sheet, cov(d_sheet, aa));

    // 4. Строчки
    for (row_y, half_w, off) in [(-7.5, 8.0, 0.0), (-0.5, 8.0, 0.0), (6.5, 5.0, -3.0)] {
        let d = rrect_sdf(lx - off, ly - row_y, half_w, 1.6, 1.6);
        over(&mut col, c.lines, cov(d, aa));
    }

    // 5. Печенька-стикер в правом верхнем углу листка
    let d_st = cookie_sdf(x, y, 45.5, 17.5, 6.6, 7, 0.16, 0.3);
    let st_shadow = [c.shadow[0], c.shadow[1], c.shadow[2], c.shadow[3] * 0.8];
    over(&mut col, st_shadow, cov(cookie_sdf(x - 0.6, y - 1.2, 45.5, 17.5, 6.6, 7, 0.16, 0.3), aa + 0.8));
    over(&mut col, c.sticker, cov(d_st, aa));

    col
}

/// Покрытие (0..1) по знаковому расстоянию.
fn cov(d: f32, aa: f32) -> f32 {
    (0.5 - d / (2.0 * aa)).clamp(0.0, 1.0)
}

/// Наложение цвета с прозрачностью (обычный режим «over», без premultiply).
fn over(dst: &mut Rgba, src: Rgba, coverage: f32) {
    let a = src[3] * coverage;
    if a <= 0.0 {
        return;
    }
    let out_a = a + dst[3] * (1.0 - a);
    for i in 0..3 {
        dst[i] = (src[i] * a + dst[i] * dst[3] * (1.0 - a)) / out_a;
    }
    dst[3] = out_a;
}

fn rrect_sdf(px: f32, py: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = px.abs() - (hw - r);
    let qy = py.abs() - (hh - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Приближённое расстояние до «печеньки» (круг с волнистым краем).
#[allow(clippy::too_many_arguments)]
fn cookie_sdf(px: f32, py: f32, cx: f32, cy: f32, radius: f32, lobes: u32, depth: f32, rot: f32) -> f32 {
    let (dx, dy) = (px - cx, py - cy);
    let a = dy.atan2(dx);
    let wave = 0.5 + 0.5 * (lobes as f32 * (a - rot)).cos();
    let r = radius * (1.0 - depth + depth * wave);
    (dx * dx + dy * dy).sqrt() - r
}
