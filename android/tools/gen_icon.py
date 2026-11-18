"""Генерирует векторную иконку-печеньку для Android (та же геометрия, что в src/icon.rs).

Запуск из папки android:  python tools/gen_icon.py
Создаёт:
  app/src/main/res/drawable/ic_launcher_foreground.xml  — цветная иконка (adaptive icon)
  app/src/main/res/drawable/ic_launcher_monochrome.xml  — силуэт для тематических иконок Android 13+
  app/src/main/res/drawable/logo.xml                    — логотип для экранов приложения
"""
import math
import os

OUT = os.path.join(os.path.dirname(__file__), "..", "app", "src", "main", "res", "drawable")

# Иконка рисуется в координатах 64×64 (как в src/icon.rs) и переносится в окно 108×108
R = 33.0                  # радиус печеньки в 108-координатах (= безопасная зона)
K = R / 30.5
C = 54.0
ANG = math.radians(-8.0)  # наклон листка

COLORS = {
    "rim": "#FFAE93F5",
    "cookie": "#FFC4AEFF",
    "shadow": "#4D402480",
    "sheet": "#FFFFFBFF",
    "lines": "#FF8B6FD6",
    "sticker": "#FFFFB4D6",
    "sticker_shadow": "#40402480",
}


def p(x, y):
    return C + (x - 32.0) * K, C + (y - 32.0) * K


def path(points):
    return "M" + " L".join(f"{x:.2f},{y:.2f}" for x, y in points) + " Z"


def cookie(cx, cy, r, lobes, depth, rot=0.0):
    n = lobes * 16
    pts = []
    for i in range(n):
        a = 2 * math.pi * i / n
        w = 0.5 + 0.5 * math.cos(lobes * (a - rot))
        rr = r * (1 - depth + depth * w)
        pts.append(p(cx + math.cos(a) * rr, cy + math.sin(a) * rr))
    return pts


def rrect(center, local, hw, hh, r, seg=6):
    """Скруглённый прямоугольник в системе координат наклонённого листка."""
    cx, cy = center
    lx, ly = local
    corners = [(hw - r, -(hh - r), -90), (hw - r, hh - r, 0), (-(hw - r), hh - r, 90), (-(hw - r), -(hh - r), 180)]
    ca, sa = math.cos(ANG), math.sin(ANG)
    pts = []
    for ox, oy, start in corners:
        for j in range(seg + 1):
            t = math.radians(start + 90 * j / seg)
            x = lx + ox + r * math.cos(t)
            y = ly + oy + r * math.sin(t)
            pts.append(p(cx + x * ca - y * sa, cy + x * sa + y * ca))
    return pts


SHEET = (31.0, 33.5)
LINES = [(-7.5, 8.0, 0.0), (-0.5, 8.0, 0.0), (6.5, 5.0, -3.0)]


def shapes():
    out = [
        (COLORS["rim"], path(cookie(32, 32, 30.5, 12, 0.075))),
        (COLORS["cookie"], path(cookie(32, 32, 28.3, 12, 0.075))),
        (COLORS["shadow"], path(rrect((SHEET[0] + 1.2, SHEET[1] + 2.4), (0, 0), 14.5, 17.5, 4.5))),
        (COLORS["sheet"], path(rrect(SHEET, (0, 0), 14.0, 17.0, 4.0))),
    ]
    for row_y, half_w, off in LINES:
        out.append((COLORS["lines"], path(rrect(SHEET, (off, row_y), half_w, 1.6, 1.6))))
    out.append((COLORS["sticker_shadow"], path(cookie(46.1, 18.7, 6.6, 7, 0.16, 0.3))))
    out.append((COLORS["sticker"], path(cookie(45.5, 17.5, 6.6, 7, 0.16, 0.3))))
    return out


def vector(paths, viewport=108, size=108, translate=0.0):
    body = "\n".join(
        f'        <path android:fillColor="{c}" android:pathData="{d}" />' for c, d in paths
    )
    return f"""<?xml version="1.0" encoding="utf-8"?>
<!-- Сгенерировано tools/gen_icon.py — не редактировать вручную -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="{size}dp" android:height="{size}dp"
    android:viewportWidth="{viewport}" android:viewportHeight="{viewport}">
    <group android:translateX="{translate}" android:translateY="{translate}">
{body}
    </group>
</vector>
"""


def main():
    os.makedirs(OUT, exist_ok=True)
    sh = shapes()
    with open(os.path.join(OUT, "ic_launcher_foreground.xml"), "w", encoding="utf-8") as f:
        f.write(vector(sh))
    # Логотип: обрезаем поля вокруг печеньки
    with open(os.path.join(OUT, "logo.xml"), "w", encoding="utf-8") as f:
        f.write(vector(sh, viewport=66, size=48, translate=-21.0))
    # Монохромная: силуэт печеньки, листок — «дырка», строчки — снова закрашены (evenOdd)
    mono = path(cookie(32, 32, 30.5, 12, 0.075)) + " " + path(rrect(SHEET, (0, 0), 14.0, 17.0, 4.0))
    for row_y, half_w, off in LINES:
        mono += " " + path(rrect(SHEET, (off, row_y), half_w, 1.6, 1.6))
    with open(os.path.join(OUT, "ic_launcher_monochrome.xml"), "w", encoding="utf-8") as f:
        f.write(f"""<?xml version="1.0" encoding="utf-8"?>
<!-- Сгенерировано tools/gen_icon.py — не редактировать вручную -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp" android:height="108dp"
    android:viewportWidth="108" android:viewportHeight="108">
    <path android:fillColor="#FF000000" android:fillType="evenOdd" android:pathData="{mono}" />
</vector>
""")
    print("Готово:", os.path.abspath(OUT))


if __name__ == "__main__":
    main()
