//! Цветные смайлики и цветные галочки задач.
//!
//! egui умеет рисовать шрифты только одним цветом, поэтому цветные emoji
//! рисуем сами: библиотека `swash` растеризует системный emoji-шрифт
//! (Segoe UI Emoji на Windows, Apple Color Emoji на macOS) в картинки.
//!
//! * В предпросмотре смайлики и галочки `- [x]` заменяются на картинки
//!   `npy-emoji://…` / `npy-check://…`, которые отдаёт наш загрузчик текстур.
//! * В редакторе смайлики рисуются поверх (прозрачного) текста.

use eframe::egui::{
    self,
    load::{LoadError, SizeHint, SizedTexture, TextureLoadResult, TextureLoader, TexturePoll},
    vec2, Color32, ColorImage, Context, TextureHandle, TextureOptions, Vec2,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use swash::{
    scale::{image::Content, Render, ScaleContext, Source, StrikeWith},
    shape::ShapeContext,
    zeno::Format,
    FontRef,
};

/// Размер растеризации (в пикселях) — с запасом для HiDPI-экранов.
const RASTER_PX: f32 = 72.0;

struct EmojiFont {
    data: Vec<u8>,
    index: usize,
}

fn load_emoji_font() -> Option<EmojiFont> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        candidates.push(std::path::Path::new(&win).join("Fonts").join("seguiemj.ttf"));
    }
    #[cfg(target_os = "macos")]
    candidates.push("/System/Library/Fonts/Apple Color Emoji.ttc".into());
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        candidates.push("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf".into());
        candidates.push("/usr/share/fonts/noto/NotoColorEmoji.ttf".into());
    }
    for path in candidates {
        if let Ok(data) = std::fs::read(&path) {
            if FontRef::from_index(&data, 0).is_some() {
                return Some(EmojiFont { data, index: 0 });
            }
        }
    }
    None
}

type Entry = Option<(TextureHandle, Vec2)>;

pub struct Emoji {
    font: Option<EmojiFont>,
    cache: Mutex<HashMap<String, Entry>>,
    ctxs: Mutex<(ShapeContext, ScaleContext)>,
}

impl Emoji {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            font: load_emoji_font(),
            cache: Mutex::new(HashMap::new()),
            ctxs: Mutex::new((ShapeContext::new(), ScaleContext::new())),
        })
    }

    /// Текстура смайлика и его пропорции (ширина/высота, нормированные к высоте 1).
    pub fn texture(&self, ctx: &Context, cluster: &str) -> Entry {
        let key = format!("e:{cluster}");
        if let Some(e) = self.cache.lock().unwrap().get(&key) {
            return e.clone();
        }
        let entry = self.raster(cluster).map(|img| {
            let aspect = vec2(img.size[0] as f32 / img.size[1] as f32, 1.0);
            (ctx.load_texture(key.clone(), img, TextureOptions::LINEAR), aspect)
        });
        self.cache.lock().unwrap().insert(key, entry.clone());
        entry
    }

    /// Текстура галочки задачи: квадратик со скруглёнными углами.
    pub fn check_texture(&self, ctx: &Context, checked: bool, fill: Color32, mark: Color32) -> Entry {
        let key = format!("c:{checked}:{}:{}", hex(fill), hex(mark));
        if let Some(e) = self.cache.lock().unwrap().get(&key) {
            return e.clone();
        }
        let img = draw_check(checked, fill, mark);
        let entry = Some((ctx.load_texture(key.clone(), img, TextureOptions::LINEAR), vec2(1.0, 1.0)));
        self.cache.lock().unwrap().insert(key, entry.clone());
        entry
    }

    fn raster(&self, text: &str) -> Option<ColorImage> {
        let f = self.font.as_ref()?;
        let font = FontRef::from_index(&f.data, f.index)?;
        let mut guard = self.ctxs.lock().unwrap();
        let (shape_cx, scale_cx) = &mut *guard;

        let mut glyphs: Vec<(u16, f32, f32, f32)> = Vec::new();
        // Script::Common — emoji относятся к «общей» письменности; так шрифт
        // применяет склейку ZWJ-последовательностей (👨‍👩‍👧) из таблицы DFLT.
        let mut shaper = shape_cx
            .builder(font)
            .script(swash::text::Script::Common)
            .size(RASTER_PX)
            .build();
        shaper.add_str(text);
        shaper.shape_with(|c| {
            for g in c.glyphs {
                glyphs.push((g.id, g.x, g.y, g.advance));
            }
        });
        if glyphs.iter().all(|g| g.0 == 0) {
            return None;
        }

        let mut scaler = scale_cx.builder(font).size(RASTER_PX).hint(false).build();
        let sources = [Source::ColorOutline(0), Source::ColorBitmap(StrikeWith::BestFit)];
        // (x, y верхнего левого угла относительно базовой линии, ширина, высота, RGBA premultiplied)
        let mut parts: Vec<(i32, i32, u32, u32, Vec<u8>)> = Vec::new();
        let mut pen = 0.0f32;
        for (id, gx, gy, adv) in glyphs {
            if id != 0 {
                if let Some(img) = Render::new(&sources).format(Format::Alpha).render(&mut scaler, id) {
                    if img.content == Content::Color && img.placement.width > 0 && img.placement.height > 0 {
                        let mut data = img.data;
                        if matches!(img.source, Source::ColorBitmap(_)) {
                            // Картинки внутри шрифта — обычный RGBA, переводим в premultiplied
                            for px in data.chunks_exact_mut(4) {
                                let a = px[3] as u32;
                                for c in &mut px[..3] {
                                    *c = (*c as u32 * a / 255) as u8;
                                }
                            }
                        }
                        let x = (pen + gx) as i32 + img.placement.left;
                        let y = -(img.placement.top) - gy as i32;
                        parts.push((x, y, img.placement.width, img.placement.height, data));
                    }
                }
            }
            pen += adv;
        }
        if parts.is_empty() {
            return None;
        }

        // Склеиваем части (на случай, если последовательность не слилась в один глиф)
        let x0 = parts.iter().map(|p| p.0).min()?;
        let y0 = parts.iter().map(|p| p.1).min()?;
        let x1 = parts.iter().map(|p| p.0 + p.2 as i32).max()?;
        let y1 = parts.iter().map(|p| p.1 + p.3 as i32).max()?;
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut canvas = vec![0u8; w * h * 4];
        for (px, py, pw, ph, data) in parts {
            for row in 0..ph as usize {
                for col in 0..pw as usize {
                    let s = (row * pw as usize + col) * 4;
                    let dx = (px - x0) as usize + col;
                    let dy = (py - y0) as usize + row;
                    let d = (dy * w + dx) * 4;
                    let sa = data[s + 3] as u32;
                    for i in 0..4 {
                        let dst = canvas[d + i] as u32;
                        canvas[d + i] = (data[s + i] as u32 + dst * (255 - sa) / 255).min(255) as u8;
                    }
                }
            }
        }
        Some(ColorImage::from_rgba_premultiplied([w, h], &canvas))
    }
}

fn hex(c: Color32) -> String {
    format!("{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

fn parse_hex(s: &str) -> Option<Color32> {
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Рисуем галочку 64×64: залитый квадратик с белой галочкой или пустая рамка.
fn draw_check(checked: bool, fill: Color32, mark: Color32) -> ColorImage {
    let n = 64usize;
    let mut px = vec![Color32::TRANSPARENT; n * n];
    let rrect = |x: f32, y: f32, half: f32, r: f32| {
        let qx = (x - 32.0).abs() - (half - r);
        let qy = (y - 32.0).abs() - (half - r);
        (qx.max(0.0).hypot(qy.max(0.0))) + qx.max(qy).min(0.0) - r
    };
    let seg = |x: f32, y: f32, ax: f32, ay: f32, bx: f32, by: f32| {
        let (pxv, pyv, bxv, byv) = (x - ax, y - ay, bx - ax, by - ay);
        let t = ((pxv * bxv + pyv * byv) / (bxv * bxv + byv * byv)).clamp(0.0, 1.0);
        (pxv - bxv * t).hypot(pyv - byv * t)
    };
    let cov = |d: f32| (0.5 - d).clamp(0.0, 1.0);
    let blend = |dst: Color32, c: Color32, a: f32| -> Color32 {
        let a = a * c.a() as f32 / 255.0;
        let l = |s: u8, d: u8| (s as f32 * a + d as f32 * (1.0 - a)) as u8;
        let out_a = a + dst.a() as f32 / 255.0 * (1.0 - a);
        Color32::from_rgba_premultiplied(
            l(c.r(), dst.r()),
            l(c.g(), dst.g()),
            l(c.b(), dst.b()),
            (out_a * 255.0) as u8,
        )
    };
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = rrect(fx, fy, 27.0, 13.0);
            let mut c = Color32::TRANSPARENT;
            if checked {
                c = blend(c, fill, cov(d));
                let dm = seg(fx, fy, 18.0, 33.0, 28.0, 43.0).min(seg(fx, fy, 28.0, 43.0, 46.0, 22.0)) - 3.6;
                c = blend(c, mark, cov(dm) * cov(d));
            } else {
                // рамка толщиной 5 px
                c = blend(c, fill, cov(d.abs() - 2.5));
            }
            px[y * n + x] = c;
        }
    }
    ColorImage { size: [n, n], pixels: px }
}

// ------------------------------------------------------------------ поиск смайликов

fn is_emoji_presentation(c: u32) -> bool {
    matches!(c,
        0x1F000..=0x1FAFF
        | 0x231A | 0x231B | 0x23E9..=0x23EC | 0x23F0 | 0x23F3 | 0x25FD | 0x25FE
        | 0x2614 | 0x2615 | 0x2648..=0x2653 | 0x267F | 0x2693 | 0x26A1 | 0x26AA | 0x26AB
        | 0x26BD | 0x26BE | 0x26C4 | 0x26C5 | 0x26CE | 0x26D4 | 0x26EA | 0x26F2 | 0x26F3
        | 0x26F5 | 0x26FA | 0x26FD | 0x2705 | 0x270A | 0x270B | 0x2728 | 0x274C | 0x274E
        | 0x2753..=0x2755 | 0x2757 | 0x2795..=0x2797 | 0x27B0 | 0x27BF | 0x2B1B | 0x2B1C
        | 0x2B50 | 0x2B55)
}

/// Символы, которые становятся смайликом только с селектором U+FE0F (❤️, ☀️, ✔️…).
fn is_emoji_with_selector(c: u32) -> bool {
    matches!(c, 0x00A9 | 0x00AE | 0x203C | 0x2049 | 0x2122 | 0x2139 | 0x2194..=0x21AA
        | 0x2300..=0x23FF | 0x24C2 | 0x25AA..=0x25FE | 0x2600..=0x27BF | 0x2934 | 0x2935
        | 0x2B05..=0x2B55 | 0x3030 | 0x303D | 0x3297 | 0x3299)
}

/// Находит смайлики (с учётом ZWJ-последовательностей, оттенков кожи, флагов).
/// Возвращает диапазоны байтов.
pub fn clusters(s: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let end_of = |k: usize| chars.get(k).map(|c| c.0).unwrap_or(s.len());
    while i < chars.len() {
        let c = chars[i].1 as u32;
        let next = chars.get(i + 1).map(|c| c.1 as u32);
        let start = i;
        let base = if (0x1F1E6..=0x1F1FF).contains(&c) {
            // флаг: пара региональных индикаторов
            if next.is_some_and(|n| (0x1F1E6..=0x1F1FF).contains(&n)) {
                i += 2;
                true
            } else {
                false
            }
        } else if matches!(chars[i].1, '0'..='9' | '#' | '*')
            && next == Some(0xFE0F)
            && chars.get(i + 2).map(|c| c.1 as u32) == Some(0x20E3)
        {
            i += 3; // 1️⃣
            true
        } else if next == Some(0xFE0E) {
            false // явно текстовое представление
        } else if is_emoji_presentation(c) || (is_emoji_with_selector(c) && next == Some(0xFE0F)) {
            i += 1;
            true
        } else {
            false
        };
        if !base {
            i = start + 1;
            continue;
        }
        // модификаторы и ZWJ-продолжения
        loop {
            let Some(&(_, ch)) = chars.get(i) else { break };
            let u = ch as u32;
            if u == 0xFE0F || (0x1F3FB..=0x1F3FF).contains(&u) || u == 0x20E3 || (0xE0020..=0xE007F).contains(&u) {
                i += 1;
            } else if u == 0x200D {
                match chars.get(i + 1) {
                    Some(&(_, n)) if is_emoji_presentation(n as u32) || is_emoji_with_selector(n as u32) => i += 2,
                    _ => break,
                }
            } else {
                break;
            }
        }
        out.push((chars[start].0, end_of(i)));
    }
    out
}

fn cluster_to_hex(s: &str) -> String {
    s.chars().map(|c| format!("{:x}", c as u32)).collect::<Vec<_>>().join("-")
}

fn hex_to_cluster(h: &str) -> Option<String> {
    h.split('-').map(|p| u32::from_str_radix(p, 16).ok().and_then(char::from_u32)).collect()
}

// ------------------------------------------------------------------ предпросмотр

/// Подготовка Markdown для предпросмотра: смайлики и галочки → картинки.
/// Внутри блоков кода и `кода в строке` ничего не меняется.
pub fn preprocess(
    md: &str,
    size: f32,
    fill: Color32,
    mark: Color32,
    outline: Color32,
    emoji: &Emoji,
    ctx: &Context,
) -> String {
    let has = |c: &str| emoji.texture(ctx, c).is_some();
    let mut out = String::with_capacity(md.len() + 64);
    let mut in_fence = false;
    let mut prev_task = false;
    let mut prev_blank = true;
    let esz = (size * 1.2).round();
    let csz = (size * 1.25).round();
    for line in md.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            out.push_str(line);
            out.push('\n');
            prev_task = false;
            prev_blank = false;
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let indent = line.len() - t.len();
        let task = ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "+ [ ] ", "+ [x] "]
            .iter()
            .find(|p| t.starts_with(**p))
            .map(|p| (p.as_bytes()[3] != b' ', &t[p.len()..]));
        if let Some((checked, rest)) = task {
            if !prev_task && !prev_blank {
                out.push('\n'); // отделяем от предыдущего абзаца
            }
            for _ in 0..indent.min(12) {
                out.push_str("&nbsp;");
            }
            let (c1, c2) = if checked { (fill, mark) } else { (outline, outline) };
            out.push_str(&format!(
                "![](npy-check://{}/{}/{}@{csz}) {}  \n",
                checked as u8,
                hex(c1),
                hex(c2),
                inline(rest, esz, &has)
            ));
            prev_task = true;
            prev_blank = false;
            continue;
        }
        if prev_task && !t.is_empty() {
            out.push('\n');
        }
        out.push_str(&inline(line, esz, &has));
        out.push('\n');
        prev_task = false;
        prev_blank = t.is_empty();
    }
    out
}

fn inline(line: &str, esz: f32, has: &dyn Fn(&str) -> bool) -> String {
    if !line.chars().any(|c| c as u32 >= 0x2000 || c == '©' || c == '®') {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + 32);
    // Делим по `…` и меняем смайлики только вне кода
    for (k, part) in line.split('`').enumerate() {
        if k > 0 {
            out.push('`');
        }
        if k % 2 == 1 {
            out.push_str(part);
            continue;
        }
        let mut last = 0;
        for (a, b) in clusters(part) {
            if !has(&part[a..b]) {
                continue; // нет в шрифте — оставляем обычным символом
            }
            out.push_str(&part[last..a]);
            out.push_str(&format!("![](npy-emoji://{}@{esz})", cluster_to_hex(&part[a..b])));
            last = b;
        }
        out.push_str(&part[last..]);
    }
    out
}

// ------------------------------------------------------------------ загрузчик текстур

pub struct Loader(pub Arc<Emoji>);

impl TextureLoader for Loader {
    fn id(&self) -> &str {
        concat!(module_path!(), "::Loader")
    }

    fn load(&self, ctx: &Context, uri: &str, _: TextureOptions, _: SizeHint) -> TextureLoadResult {
        let (body, size) = match uri.rsplit_once('@') {
            Some((b, s)) => (b, s.parse::<f32>().unwrap_or(18.0)),
            None => (uri, 18.0),
        };
        let entry = if let Some(h) = body.strip_prefix("npy-emoji://") {
            let cluster = hex_to_cluster(h).ok_or_else(|| LoadError::Loading("bad emoji".into()))?;
            self.0.texture(ctx, &cluster)
        } else if let Some(rest) = body.strip_prefix("npy-check://") {
            let mut it = rest.split('/');
            let checked = it.next() == Some("1");
            let fill = it.next().and_then(parse_hex).unwrap_or(Color32::GRAY);
            let mark = it.next().and_then(parse_hex).unwrap_or(Color32::WHITE);
            self.0.check_texture(ctx, checked, fill, mark)
        } else {
            return Err(LoadError::NotSupported);
        };
        match entry {
            Some((tex, aspect)) => Ok(TexturePoll::Ready {
                texture: SizedTexture::new(tex.id(), egui::vec2(size * aspect.x.min(2.5), size)),
            }),
            None => Err(LoadError::Loading("Нет цветного emoji-шрифта".into())),
        }
    }

    fn forget(&self, _uri: &str) {}

    fn forget_all(&self) {
        self.0.cache.lock().unwrap().clear();
    }

    fn byte_size(&self) -> usize {
        self.0.cache.lock().unwrap().len() * (RASTER_PX as usize * RASTER_PX as usize * 4)
    }
}
