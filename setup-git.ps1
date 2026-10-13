# Inicializa el repositorio git (solo hace falta una vez)
Set-Location $PSScriptRoot
if (Test-Path ".git") { Write-Host "Ya es un repositorio git"; exit 0 }
git init -b main
git add -A
git commit -m "Esqueleto inicial: DLL .asi, deteccion de version, sistema de hooks y CTimer::Update"
Write-Host "Repositorio creado." -ForegroundColor Green
