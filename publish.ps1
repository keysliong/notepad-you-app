# Обновить сайт на GitHub Pages: .\publish.ps1
# Сначала соберите программу (.\build.ps1) — он положит свежий установщик в site\download.
# Сайт: https://keysliong.github.io/notepad-you/
$ErrorActionPreference = "Continue"  # git пишет прогресс в stderr — это не ошибка
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')

# Кнопки «Скачать» ведут в последний релиз GitHub, а GitHub считает скачивания для счётчика на сайте.
# Если файлы в site\download изменились, выпускаем НОВЫЙ релиз со всеми файлами:
# перезаливать старый нельзя — у перезалитого файла счётчик обнуляется (сайт суммирует все релизы).
$repo = "keysliong/notepad-you-app"
$files = Get-ChildItem (Join-Path $PSScriptRoot "site\download") -File
$remote = @{}
gh api "repos/$repo/releases/latest" --jq '.assets[]|[.name,.digest]|@tsv' | ForEach-Object {
    $p = $_ -split "`t", 2; $remote[$p[0]] = $p[1]
}
$changed = $files | Where-Object { $remote[$_.Name] -ne ("sha256:" + (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()) }
if ($changed) {
    $tag = "v" + (Get-Date -Format "yyyy.MM.dd-HHmm")
    Write-Host "Новые файлы: $($changed.Name -join ', ') → релиз $tag" -ForegroundColor Cyan
    gh release create $tag -R $repo --title "Notepad You $tag" --notes "Windows, Mac и Android. Сайт: https://keysliong.github.io/notepad-you/" $($files.FullName)
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

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
