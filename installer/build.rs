//! Встраивает иконку и сведения о версии в NotepadYou-Setup.exe.
//! Иконку берём из ../icon/NotepadYou.ico (её создаёт сборка основного приложения).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../icon/NotepadYou.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    if std::fs::copy(manifest.join("../icon/NotepadYou.ico"), out.join("app.ico")).is_err() {
        println!("cargo:warning=Нет icon/NotepadYou.ico — сначала соберите основное приложение");
        return;
    }
    let rc = r#"#pragma code_page(65001)
1 ICON "app.ico"
1 VERSIONINFO
FILEVERSION 1,1,0,0
PRODUCTVERSION 1,1,0,0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "041904B0"
    BEGIN
      VALUE "FileDescription", "Установщик Notepad You"
      VALUE "ProductName", "Notepad You"
      VALUE "FileVersion", "1.1.0"
      VALUE "ProductVersion", "1.1.0"
      VALUE "OriginalFilename", "NotepadYou-Setup.exe"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x419, 1200
  END
END
"#;
    std::fs::write(out.join("app.rc"), rc).unwrap();
    // Относительные пути — windres ломается на пробелах в пути проекта
    let status = std::process::Command::new("windres")
        .current_dir(&out)
        .args(["-i", "app.rc", "-o", "app_res.o", "-O", "coff"])
        .status();
    match status {
        Ok(s) if s.success() => println!("cargo:rustc-link-arg-bins={}", out.join("app_res.o").display()),
        other => println!("cargo:warning=Иконка не встроена (нужен windres из MinGW): {other:?}"),
    }
}
