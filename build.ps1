# Сборка Notepad You: .\build.ps1
# Готовые файлы появятся рядом:
#   .\NotepadYou.exe        — сама программа
#   .\NotepadYou-Setup.exe  — установщик (программа вшита внутрь)
#   .\NotepadYou-Uninstall.exe — удалятор (тот же файл: по имени включает режим удаления)
$ErrorActionPreference = "Continue"  # cargo пишет прогресс в stderr — это не ошибка
$mingw = "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"
$env:Path = "$mingw;$env:USERPROFILE\.cargo\bin;$env:Path"

Set-Location $PSScriptRoot
# 1) программа (заодно создаёт icon\NotepadYou.ico)
cargo build --release -p notepad-you
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# 2) установщик — вшивает только что собранный NotepadYou.exe
cargo build --release -p notepad-you-setup
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Copy-Item "target\release\NotepadYou.exe" "NotepadYou.exe" -Force
Copy-Item "target\release\NotepadYou-Setup.exe" "NotepadYou-Setup.exe" -Force
Copy-Item "target\release\NotepadYou-Setup.exe" "NotepadYou-Uninstall.exe" -Force
# 3) сайт: свежий установщик, удалятор и иконка
New-Item -ItemType Directory -Force "site\download", "site\img" | Out-Null
Copy-Item "NotepadYou-Setup.exe", "NotepadYou-Uninstall.exe" "site\download\" -Force
Copy-Item "icon\NotepadYou.png" "site\img\icon.png" -Force
Write-Host "Готово: $PSScriptRoot\NotepadYou.exe" -ForegroundColor Green
Write-Host "Готово: $PSScriptRoot\NotepadYou-Setup.exe" -ForegroundColor Green
Write-Host "Готово: $PSScriptRoot\NotepadYou-Uninstall.exe" -ForegroundColor Green
