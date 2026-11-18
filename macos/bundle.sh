#!/usr/bin/env bash
# Сборка Notepad You для macOS: универсальный .app (Apple Silicon + Intel) и .dmg.
# Запускается на Mac (или в GitHub Actions, см. .github/workflows/macos.yml):
#   bash macos/bundle.sh
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP="dist/Notepad You.app"

echo "==> Сборка версии $VERSION"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release -p notepad-you --target aarch64-apple-darwin
cargo build --release -p notepad-you --target x86_64-apple-darwin

echo "==> Пакет .app"
rm -rf dist && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/NotepadYou" \
  target/aarch64-apple-darwin/release/NotepadYou \
  target/x86_64-apple-darwin/release/NotepadYou
sed "s/__VERSION__/$VERSION/g" macos/Info.plist > "$APP/Contents/Info.plist"

echo "==> Иконка AppIcon.icns"
SRC=icon/NotepadYou-1024.png
SET=dist/AppIcon.iconset
mkdir -p "$SET"
for s in 16 32 128 256 512; do
  sips -z $s $s "$SRC" --out "$SET/icon_${s}x${s}.png" >/dev/null
  d=$((s * 2))
  sips -z $d $d "$SRC" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$SET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$SET"

echo "==> Подпись (ad-hoc, без сертификата Apple)"
codesign --force --deep --sign - "$APP"

echo "==> Образ диска .dmg"
STAGE=dist/dmg
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Notepad You" -srcfolder "$STAGE" -ov -format UDZO "dist/NotepadYou.dmg"
rm -rf "$STAGE"

echo "==> Готово: dist/NotepadYou.dmg"
ls -lh dist
