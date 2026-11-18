# Обновить сайт на GitHub Pages: .\publish.ps1
# Сначала соберите программу (.\build.ps1) — он положит свежий установщик в site\download.
# Сайт: https://keysliong.github.io/notepad-you/
$ErrorActionPreference = "Continue"  # git пишет прогресс в stderr — это не ошибка
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')

Set-Location (Join-Path $PSScriptRoot "site")
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -eq 0) {
    Write-Host "Изменений нет — сайт уже актуален." -ForegroundColor Yellow
    exit 0
}
git commit -q -m "Обновление сайта $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
git push -q
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Отправлено! Через 1–2 минуты обновится https://keysliong.github.io/notepad-you/" -ForegroundColor Green
