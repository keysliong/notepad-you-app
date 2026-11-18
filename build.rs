//! Встраивает иконку и сведения о версии в NotepadYou.exe.

#[path = "src/icon.rs"]
#[allow(dead_code)]
mod icon;

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // Для macOS: большая PNG, из которой macos/bundle.sh делает AppIcon.icns
        let icon_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("icon");
        let _ = std::fs::create_dir_all(&icon_dir);
        write_if_changed(&icon_dir.join("NotepadYou-1024.png"), &make_png(1024));
        return;
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico_bytes = make_ico(&[16, 24, 32, 48, 64, 256]);
    std::fs::write(out.join("app.ico"), &ico_bytes).unwrap();

    // Копия иконки в папку проекта icon/ (.ico и .png)
    let icon_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("icon");
    let _ = std::fs::create_dir_all(&icon_dir);
    write_if_changed(&icon_dir.join("NotepadYou.ico"), &ico_bytes);
    write_if_changed(&icon_dir.join("NotepadYou.png"), &make_png(512));

    let rc = r#"#pragma code_page(65001)
1 ICON "app.ico"
1 VERSIONINFO
FILEVERSION 1,0,0,0
PRODUCTVERSION 1,0,0,0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "041904B0"
    BEGIN
      VALUE "FileDescription", "Notepad You — блокнот в стиле Material You"
      VALUE "ProductName", "Notepad You"
      VALUE "FileVersion", "1.0.0"
      VALUE "ProductVersion", "1.0.0"
      VALUE "OriginalFilename", "NotepadYou.exe"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x419, 1200
  END
END
"#;
    std::fs::write(out.join("app.rc"), rc).unwrap();

    // windres запускаем из OUT_DIR с относительными путями:
    // иначе он ломается на пробелах в пути проекта.
    let status = std::process::Command::new("windres")
        .current_dir(&out)
        .args(["-i", "app.rc", "-o", "app_res.o", "-O", "coff"])
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-link-arg-bins={}", out.join("app_res.o").display());
        }
        other => println!("cargo:warning=Иконка .exe не встроена (нужен windres из MinGW): {other:?}"),
    }
}

fn write_if_changed(path: &std::path::Path, data: &[u8]) {
    if std::fs::read(path).ok().as_deref() != Some(data) {
        let _ = std::fs::write(path, data);
    }
}

/// Простой PNG-кодировщик (deflate без сжатия) — без внешних зависимостей.
fn make_png(n: u32) -> Vec<u8> {
    let rgba = icon::icon_rgba(n);
    let mut raw = Vec::with_capacity((n * (n * 4 + 1)) as usize);
    for y in 0..n as usize {
        raw.push(0); // фильтр None
        raw.extend_from_slice(&rgba[y * n as usize * 4..(y + 1) * n as usize * 4]);
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push(if i + 1 == chunks.len() { 1 } else { 0 });
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&n.to_be_bytes());
    ihdr.extend_from_slice(&n.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &z);
    png_chunk(&mut png, b"IEND", &[]);
    png
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in &out[start..] {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

/// Собирает .ico из нескольких размеров (32-бит BMP).
fn make_ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&n| bmp_entry(n)).collect();
    let mut f = Vec::new();
    f.extend_from_slice(&[0, 0, 1, 0]);
    f.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (n, img) in sizes.iter().zip(&images) {
        let b = if *n >= 256 { 0 } else { *n as u8 };
        f.extend_from_slice(&[b, b, 0, 0]);
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&32u16.to_le_bytes());
        f.extend_from_slice(&(img.len() as u32).to_le_bytes());
        f.extend_from_slice(&offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        f.extend_from_slice(&img);
    }
    f
}

fn bmp_entry(n: u32) -> Vec<u8> {
    let rgba = icon::icon_rgba(n);
    let mut b = Vec::new();
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
