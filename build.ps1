# Compila gta-rs y lo copia a la carpeta del juego como gta_rs.asi
# Uso:  .\build.ps1                (release)
#       .\build.ps1 -GameDir "D:\otra\ruta\GTA San Andreas"
param(
    [string]$GameDir = "E:\SteamLibrary\steamapps\common\Grand Theft Auto San Andreas"
)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# El juego es de 32 bits
rustup target add i686-pc-windows-msvc | Out-Null

# Target de 32 bits y CRT estatico (el .asi no depende de los redistribuibles de VC++)
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --target i686-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "La compilacion ha fallado" }

$out = "target\i686-pc-windows-msvc\release"
if (-not (Test-Path "$GameDir\gta_sa.exe")) {
    Write-Warning "No encuentro gta_sa.exe en '$GameDir' (falta el downgrade a 1.0 US?). Copio igualmente."
}
Copy-Item "$out\gta_rs.dll" "$GameDir\gta_rs.asi" -Force
if (Test-Path "$out\gta_rs.pdb") { Copy-Item "$out\gta_rs.pdb" "$GameDir\gta_rs.pdb" -Force }
if (-not (Test-Path "$GameDir\gta_rs.ini")) { Copy-Item "gta_rs.ini" "$GameDir\gta_rs.ini" }

Write-Host "OK: gta_rs.asi copiado a $GameDir" -ForegroundColor Green
Write-Host "Arranca el juego y mira $GameDir\gta_rs.log"
