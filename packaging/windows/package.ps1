# Compila ForgeRender e crea le due distribuzioni Windows:
#   dist\ForgeRender-<versione>-portable-x64.zip   (nessuna installazione)
#   dist\ForgeRender-<versione>-setup-x64.exe      (installer, richiede Inno Setup 6)
#
# Uso (dalla cartella del repository, in PowerShell):
#   powershell -ExecutionPolicy Bypass -File packaging\windows\package.ps1
param(
    [switch]$SkipBuild,
    [switch]$SkipInstaller,
    # Include il convertitore STEP (compila Open CASCADE: decine di minuti la prima volta).
    [switch]$WithStep,
    # Cartella di Open Image Denoise (scaricato automaticamente se manca).
    [string]$OidnDir = ""
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
Write-Host "ForgeRender $version"

if (-not $SkipBuild) {
    cargo build --release -p forge_app -p forge_cli
    if ($LASTEXITCODE -ne 0) { throw "compilazione fallita" }
    if ($WithStep) {
        # Convertitore STEP (Open CASCADE): la prima compilazione richiede decine di minuti.
        cargo build --release -p forge_step
        if ($LASTEXITCODE -ne 0) { throw "compilazione del convertitore STEP fallita" }
    }
}

$bin = Join-Path $root "target\release"

# --- Open Image Denoise (denoiser, Apache 2.0): solo le librerie per CPU e GPU NVIDIA (CUDA) ---
$oidnVersion = "2.3.3"
if (-not $OidnDir) { $OidnDir = Join-Path $root "target\oidn-$oidnVersion.x64.windows" }
if (-not (Test-Path (Join-Path $OidnDir "bin\OpenImageDenoise.dll"))) {
    $oidnZip = Join-Path $env:TEMP "oidn-$oidnVersion.x64.windows.zip"
    Write-Host "Download di Open Image Denoise $oidnVersion..."
    Invoke-WebRequest -Uri "https://github.com/RenderKit/oidn/releases/download/v$oidnVersion/oidn-$oidnVersion.x64.windows.zip" -OutFile $oidnZip
    $oidnParent = Split-Path $OidnDir -Parent
    Expand-Archive -Path $oidnZip -DestinationPath $oidnParent -Force
    $extracted = Join-Path $oidnParent "oidn-$oidnVersion.x64.windows"
    if ($extracted -ne $OidnDir) { Move-Item $extracted $OidnDir -Force }
}
$oidnDlls = @("OpenImageDenoise.dll", "OpenImageDenoise_core.dll", "OpenImageDenoise_device_cpu.dll", "OpenImageDenoise_device_cuda.dll", "tbb12.dll")
$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null

# --- Portable ---
$portableName = "ForgeRender-$version-portable-x64"
$staging = Join-Path $dist $portableName
if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Path $staging | Out-Null
Copy-Item (Join-Path $bin "forgerender.exe") $staging
Copy-Item (Join-Path $bin "forgerender-cli.exe") $staging
$stepExe = Join-Path $bin "forgerender-step.exe"
# Solo se richiesto: un forgerender-step.exe rimasto da una compilazione precedente non entra nel pacchetto.
if ($WithStep -and (Test-Path $stepExe)) { Copy-Item $stepExe $staging } else { Write-Host "Pacchetto senza convertitore STEP (usa -WithStep per includerlo)" }
Copy-Item (Join-Path $PSScriptRoot "LEGGIMI.txt") $staging
New-Item -ItemType Directory -Force -Path (Join-Path $staging "licenze") | Out-Null
Copy-Item (Join-Path $root "packaging\licenses\*") (Join-Path $staging "licenze")
foreach ($dll in $oidnDlls) { Copy-Item (Join-Path $OidnDir "bin\$dll") $staging }
$oidnLic = Join-Path $staging "licenze\OpenImageDenoise"
New-Item -ItemType Directory -Force -Path $oidnLic | Out-Null
Copy-Item (Join-Path $OidnDir "doc\LICENSE.txt"), (Join-Path $OidnDir "doc\third-party-programs*.txt") $oidnLic
# Cartelle della libreria (HDRI, materiali, texture): nella portable stanno accanto al programma.
foreach ($sub in @("HDRI", "Materiali", "Texture")) {
    $d = Join-Path $staging "Libreria\$sub"
    New-Item -ItemType Directory -Force -Path $d | Out-Null
}
Set-Content -Path (Join-Path $staging "Libreria\HDRI\LEGGIMI.txt") -Value "Metti qui le mappe HDRI (.hdr, .exr): compaiono con l'anteprima nella scheda Ambiente."
Set-Content -Path (Join-Path $staging "Libreria\Materiali\LEGGIMI.txt") -Value "Materiali della libreria (una cartella per materiale). Usa Libreria -> Importa cartella/zip oppure scarica da ambientCG / Poly Haven."
Set-Content -Path (Join-Path $staging "Libreria\Texture\LEGGIMI.txt") -Value "Texture sciolte da usare negli slot dei materiali."
# Scene di esempio e progetti di benchmark pronti (samples\...\bench_*.frproject): lo zip
# basta da solo per lanciare i benchmark, senza Python né il repository. Le texture estratte
# dai .glb (*_texture) non si copiano: il programma le ricrea all'apertura del progetto.
# Solo nella portable: nell'installata (Program Files) il programma non può scriverle.
$samples = Join-Path $staging "samples"
Copy-Item -Recurse (Join-Path $root "samples") $samples
Get-ChildItem -Path $samples -Recurse -Directory -Filter "*_texture" | Remove-Item -Recurse -Force
Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $samples ".gitignore")
foreach ($b in @("bench_ad", "bench_intera", "bench_vicina")) {
    $f = Join-Path $samples "scene\cocktail-ghiaccio-secco\$b.frproject"
    if (-not (Test-Path $f)) { throw "manca il progetto di benchmark $f" }
}
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
        & $iscc "/DAppVersion=$version" "/DBinDir=$bin" "/DOidnDir=$OidnDir" (Join-Path $PSScriptRoot "forgerender.iss")
        if ($LASTEXITCODE -ne 0) { throw "creazione dell'installer fallita" }
        Write-Host "Installer: $dist\ForgeRender-$version-setup-x64.exe"
    }
}
