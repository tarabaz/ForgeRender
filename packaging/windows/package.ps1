# Compila ForgeRender e crea le due distribuzioni Windows:
#   dist\ForgeRender-<versione>-portable-x64.zip   (nessuna installazione)
#   dist\ForgeRender-<versione>-setup-x64.exe      (installer, richiede Inno Setup 6)
#
# Uso (dalla cartella del repository, in PowerShell):
#   powershell -ExecutionPolicy Bypass -File packaging\windows\package.ps1
param(
    [switch]$SkipBuild,
    [switch]$SkipInstaller
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "ForgeRender $version"

if (-not $SkipBuild) {
    cargo build --release -p forge_app -p forge_cli
    if ($LASTEXITCODE -ne 0) { throw "compilazione fallita" }
}

$bin = Join-Path $root "target\release"
$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null

# --- Portable ---
$portableName = "ForgeRender-$version-portable-x64"
$staging = Join-Path $dist $portableName
if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Path $staging | Out-Null
Copy-Item (Join-Path $bin "forgerender.exe") $staging
Copy-Item (Join-Path $bin "forgerender-cli.exe") $staging
Copy-Item (Join-Path $PSScriptRoot "LEGGIMI.txt") $staging
# Marcatore della modalità portable: i log restano nella cartella del programma.
Set-Content -Path (Join-Path $staging "portable.txt") -Value "Versione portable di ForgeRender: i log vengono scritti nella sottocartella logs."
$zip = Join-Path $dist "$portableName.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path "$staging\*" -DestinationPath $zip
Write-Host "Portable: $zip"

# --- Installer ---
if (-not $SkipInstaller) {
    $iscc = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 6\ISCC.exe",
        "${env:LOCALAPPDATA}\Programs\Inno Setup 6\ISCC.exe"
    ) | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $iscc) {
        Write-Warning "Inno Setup 6 non trovato: installer non creato (scaricalo da https://jrsoftware.org/isdl.php)"
    } else {
        & $iscc "/DAppVersion=$version" "/DBinDir=$bin" (Join-Path $PSScriptRoot "forgerender.iss")
        if ($LASTEXITCODE -ne 0) { throw "creazione dell'installer fallita" }
        Write-Host "Installer: $dist\ForgeRender-$version-setup-x64.exe"
    }
}
