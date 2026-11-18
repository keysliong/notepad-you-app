#!/usr/bin/env bash
# Проверка APK в эмуляторе: проходит по экранам, снимает скриншоты, ищет падения.
# Запускается из .github/workflows/android-test.yml
set -u
APK=android/app/build/outputs/apk/release/app-release.apk
OUT=shots
mkdir -p "$OUT"

adb install -r "$APK"
adb logcat -c
adb shell am start -n io.github.notepadyou/.MainActivity
sleep 8

shot() { adb exec-out screencap -p > "$OUT/$1.png"; echo "снимок: $1"; }

# Нажать на элемент по тексту или описанию (через дерево доступности)
tap() {
  adb shell uiautomator dump /sdcard/ui.xml > /dev/null
  adb pull /sdcard/ui.xml ui.xml > /dev/null
  local xy
  xy=$(python3 - "$1" <<'EOF'
import re, sys
needle = sys.argv[1]
xml = open("ui.xml", encoding="utf-8").read()
for node in re.findall(r"<node [^>]*>", xml):
    text = re.search(r'text="([^"]*)"', node).group(1)
    desc = re.search(r'content-desc="([^"]*)"', node).group(1)
    if needle in text or needle in desc:
        x1, y1, x2, y2 = map(int, re.search(r'bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"', node).groups())
        print((x1 + x2) // 2, (y1 + y2) // 2)
        break
EOF
)
  if [ -z "$xy" ]; then echo "не найдено: $1"; return 1; fi
  adb shell input tap $xy
  echo "нажато: $1 ($xy)"
}

shot 1-onboarding
tap "Далее" && sleep 2 && shot 2-onboarding-markdown
tap "Пропустить" && sleep 3 && shot 3-list
tap "Добро пожаловать" && sleep 3 && shot 4-editor
tap "Просмотр" && sleep 2 && shot 5-preview
adb shell input keyevent KEYCODE_BACK && sleep 2
tap "Настройки" && sleep 2 && shot 6-settings
tap "Коралл" && sleep 2 && shot 7-settings-coral
adb shell cmd uimode night yes && sleep 4 && shot 8-settings-dark
adb shell input keyevent KEYCODE_BACK && sleep 2 && shot 9-list-dark

# Падения приложения
adb logcat -d -b crash > "$OUT/crash.txt"
if grep -q "io.github.notepadyou" "$OUT/crash.txt"; then
  echo "::error::Приложение упало, см. crash.txt"
  cat "$OUT/crash.txt"
  exit 1
fi
adb shell pidof io.github.notepadyou > /dev/null && echo "Приложение работает, падений нет ✓"
