#!/usr/bin/env bash
# Запись видео с экрана эмулятора для рекламного ролика.
# Каждая сцена — отдельный ролик rec/<сцена>.mp4, плюс журнал действий rec/<сцена>.log
# (время от начала записи в секундах + действие), чтобы потом вырезать паузы.
# Запускается из .github/workflows/android-record.yml
set -u
set -f   # без подстановки * и [ ] — они нужны в Markdown
APK=android/app/build/outputs/apk/release/app-release.apk
OUT=rec
mkdir -p "$OUT"

adb install -r "$APK"

# Индикаторы касаний и чистая строка состояния (демо-режим SystemUI)
adb shell settings put system show_touches 1
adb shell settings put global sysui_demo_allowed 1
adb shell am broadcast -a com.android.systemui.demo -e command enter
adb shell am broadcast -a com.android.systemui.demo -e command clock -e hhmm 1200
adb shell am broadcast -a com.android.systemui.demo -e command battery -e level 100 -e plugged false
adb shell am broadcast -a com.android.systemui.demo -e command network -e wifi show -e level 4
adb shell am broadcast -a com.android.systemui.demo -e command network -e mobile show -e datatype none -e level 4
adb shell am broadcast -a com.android.systemui.demo -e command notifications -e visible false

adb shell am start -n io.github.notepadyou/.MainActivity
sleep 10

SCENE=""
T0=0
now() { date +%s.%N; }
log() { echo "$(awk "BEGIN{printf \"%.3f\", $(now) - $T0}") $*" >> "$OUT/$SCENE.log"; echo "[$SCENE] $*"; }

start_rec() {
  SCENE=$1
  : > "$OUT/$SCENE.log"
  adb shell rm -f /sdcard/rec.mp4
  adb shell screenrecord --size 720x1600 --bit-rate 12000000 /sdcard/rec.mp4 &
  REC_PID=$!
  sleep 1.2
  T0=$(now)
  log start
}

stop_rec() {
  log stop
  adb shell pkill -INT screenrecord
  wait $REC_PID 2>/dev/null
  sleep 2
  adb pull /sdcard/rec.mp4 "$OUT/$SCENE.mp4" > /dev/null
  echo "записано: $SCENE"
}

# Найти элемент по тексту / описанию / классу и вернуть "x1 y1 x2 y2"
bounds() {
  adb shell uiautomator dump /sdcard/ui.xml > /dev/null
  adb pull /sdcard/ui.xml ui.xml > /dev/null
  python3 - "$1" <<'EOF'
import re, sys
needle = sys.argv[1]
xml = open("ui.xml", encoding="utf-8").read()
for node in re.findall(r"<node [^>]*>", xml):
    text = re.search(r'text="([^"]*)"', node).group(1)
    desc = re.search(r'content-desc="([^"]*)"', node).group(1)
    cls = re.search(r'class="([^"]*)"', node).group(1)
    if needle in text or needle in desc or needle == cls:
        print(*re.search(r'bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"', node).groups())
        break
EOF
}

tap_xy() { log "tap $1 $2 $3"; adb shell input tap $1 $2; }

tap_text() {
  local b; b=$(bounds "$1")
  if [ -z "$b" ]; then sleep 1.5; b=$(bounds "$1"); fi
  if [ -z "$b" ]; then echo "не найдено: $1"; return 1; fi
  set -- $b "$1"
  tap_xy $(( ($1 + $3) / 2 )) $(( ($2 + $4) / 2 )) "$5"
}

swipe() { log "swipe $*"; adb shell input swipe "$@"; }

# Набор текста «как человек»: по словам. Только латиница (input text не умеет кириллицу).
type_words() {
  local first=1
  for w in $1; do
    if [ $first -eq 0 ]; then adb shell input keyevent 62; fi
    case "$w" in
      -) adb shell input keyevent 69 ;;     # минус
      +) adb shell input keyevent 81 ;;     # плюс
      *) adb shell "input text '$w'" ;;
    esac
    first=0
    sleep 0.12
  done
}
enter() { adb shell input keyevent 66; }

# ── 1. Приветствие ───────────────────────────────────────────────
start_rec onboarding
sleep 2.2
tap_xy 894 2211 "Далее"
sleep 2.2
tap_xy 894 2211 "Далее"
sleep 2.4
tap_xy 894 2211 "Начать"
sleep 3
stop_rec

# ── 2. Открыть заметку, прокрутить, просмотр, отметить задачу ────
start_rec note
sleep 1.5
tap_text "Добро пожаловать" || tap_xy 540 690 "заметка"
sleep 2.2
swipe 540 1700 540 1100 700
sleep 1.2
swipe 540 1100 540 1700 700
sleep 1.2
tap_text "Просмотр" || tap_xy 819 369 "Просмотр"
sleep 2.5
B=$(bounds "Написать первую заметку")
if [ -n "$B" ]; then set -- $B; tap_xy $(( $1 - 55 )) $(( ($2 + $4) / 2 )) "галочка"; fi
sleep 1.5
B=$(bounds "Выбрать любимый цвет")
if [ -n "$B" ]; then set -- $B; tap_xy $(( $1 - 55 )) $(( ($2 + $4) / 2 )) "галочка2"; fi
sleep 2
log back; adb shell input keyevent KEYCODE_BACK
sleep 2
stop_rec

# ── 3. Новая заметка: набор Markdown и просмотр ──────────────────
start_rec typing
sleep 1.2
tap_text "Новая заметка" || tap_xy 912 2169 "FAB"
sleep 2
tap_text "Начните писать" || tap_xy 540 700 "поле"
sleep 1.2
log type
type_words "#%sNotepad%sYou"   # %s — пробел без разбиения на слова
enter; enter
type_words "**Material You** + Markdown"
enter; enter
type_words "- [x] Android"
enter
type_words "- [ ] Windows"
enter
type_words "- [ ] macOS"
log typed
sleep 1.2
adb shell input keyevent KEYCODE_BACK   # спрятать клавиатуру
sleep 1
tap_text "Просмотр" || tap_xy 819 369 "Просмотр"
sleep 3
log back; adb shell input keyevent KEYCODE_BACK
sleep 2
stop_rec

# ── 4. Удаление свайпом и «Вернуть» ──────────────────────────────
start_rec swipe
sleep 1.5
B=$(bounds "Android")
[ -z "$B" ] && B=$(bounds "Новая заметка")
if [ -n "$B" ]; then set -- $B; Y=$(( ($2 + $4) / 2 )); else Y=690; fi
swipe 150 $Y 1000 $Y 350
sleep 1.6
tap_text "Вернуть"
sleep 2.5
stop_rec

# ── 5. Настройки: темы, тёмная тема, размер текста ───────────────
start_rec settings
sleep 1.2
tap_text "Настройки" || tap_xy 858 210 "Настройки"
sleep 2.2
for c in "351 Коралл" "531 Океан" "705 Мята" "885 Роза"; do
  set -- $c
  tap_xy $1 1050 "$2"
  sleep 1.3
done
tap_xy 171 1263 "Янтарь"; sleep 1.3
tap_xy 171 1050 "Лаванда"; sleep 1.6
tap_text "Тёмная" || tap_xy 834 1680 "Тёмная"
sleep 2.4
swipe 540 1900 540 1000 600
sleep 1.5
B=$(bounds "android.widget.SeekBar")
if [ -n "$B" ]; then
  set -- $B
  Y=$(( ($2 + $4) / 2 )); X0=$(( $1 + ($3 - $1) * 5 / 14 )); X1=$(( $1 + ($3 - $1) * 12 / 14 ))
  swipe $X0 $Y $X1 $Y 1200
  sleep 1.6
  swipe $X1 $Y $X0 $Y 900
  sleep 1.2
fi
swipe 540 1000 540 1900 600
sleep 1.2
log back; adb shell input keyevent KEYCODE_BACK
sleep 2.5
stop_rec

# ── 6. Тёмный список и снова светлая тема ────────────────────────
start_rec list
sleep 2
tap_text "Добро пожаловать" || tap_xy 540 690 "заметка"
sleep 2.5
tap_text "Просмотр" || tap_xy 819 369 "Просмотр"
sleep 2.5
log back; adb shell input keyevent KEYCODE_BACK
sleep 2
stop_rec

adb logcat -d -b crash > "$OUT/crash.txt"
ls -la "$OUT"
