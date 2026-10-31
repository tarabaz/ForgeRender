<#
.SYNOPSIS
    Aggiorna ForgeRender da GitHub e lo compila in locale, con un solo comando.

.DESCRIPTION
    1. Controlla di essere nel repository ForgeRender, sul branch giusto, senza modifiche
       locali non committate (se ce ne sono si ferma e non tocca nulla).
    2. git pull --ff-only dal branch di sviluppo (se fallisce si ferma: niente build obsolete).
    3. cargo build --release -p forge_app -p forge_cli -p forge_mcp_bridge (cache Cargo riutilizzata).
    4. Convertitore STEP: cargo build --release -p forge_step (Open CASCADE; serve CMake). La prima
       volta richiede decine di minuti, poi resta nella cache. Se CMake manca o la compilazione
       fallisce, il resto prosegue e il riepilogo spiega cosa fare. -SkipStep lo salta.
    5. Pacchetto portable con packaging\windows\package.ps1 -SkipBuild (nessuna seconda build).

    Non usa GitHub Actions, non scarica artefatti, non fa commit, push, reset o stash.

.EXAMPLE
    .\Build-ForgeRender.ps1
.EXAMPLE
    .\Build-ForgeRender.ps1 -SkipInstaller      # solo portable, senza Inno Setup
.EXAMPLE
    .\Build-ForgeRender.ps1 -SkipStep          # senza convertitore STEP (più veloce)
.EXAMPLE
    .\Build-ForgeRender.cmd                    # se Windows blocca gli script (Execution Policy)

.NOTES
    Execution Policy: se PowerShell rifiuta di eseguire lo script ("l'esecuzione di script è
    disabilitata nel sistema"), usa Build-ForgeRender.cmd oppure, nella stessa finestra,
        Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
    e poi .\Build-ForgeRender.ps1. Entrambi valgono solo per quel processo: le impostazioni
    di sicurezza di Windows non vengono cambiate in modo permanente.
#>
[CmdletBinding()]
param(
    # Crea solo il pacchetto portable (l'installer richiede Inno Setup 6).
    [switch]$SkipInstaller,
    # Non compilare il convertitore STEP (import di STEP e SolidWorks).
    [switch]$SkipStep,
    # Non aprire la cartella dist al termine.
    [switch]$NoOpen
)

$ErrorActionPreference = 'Stop'

$Branch = 'claude/festive-heisenberg-d2ow6z'
$Root = $PSScriptRoot

function Step([string]$text) { Write-Host "`n==> $text" -ForegroundColor Cyan }
function Fail([string]$text) {
    Write-Host "`nERRORE: $text" -ForegroundColor Red
    Write-Host 'Procedura interrotta: nessun pacchetto creato con questa esecuzione.' -ForegroundColor Red
    exit 1
}
# Esegue un comando esterno, ne restituisce l'output e si ferma se il codice di uscita non è 0.
# Conta solo il codice di uscita: git scrive anche messaggi normali su stderr (es. "From https://…"),
# che con 'Stop' Windows PowerShell 5.1 trasformerebbe in errori.
function Run([string]$what, [scriptblock]$cmd) {
    $ErrorActionPreference = 'Continue'
    $out = & $cmd 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) {
        if ($out) { $out | ForEach-Object { Write-Host "    $_" } }
        Fail "$what (codice di uscita $LASTEXITCODE)"
    }
    return $out
}

Set-Location $Root
$started = Get-Date

# ------------------------------------------------------------------ 1. Controlli
Step 'Controlli preliminari'
foreach ($tool in 'git', 'cargo') {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Fail "$tool non trovato nel PATH (installalo o riapri PowerShell dopo l'installazione)"
    }
}

# Repository giusto: radice Git = cartella dello script, remoto tarabaz/ForgeRender (prima
# tarabaz/RenderingEngine: dopo il cambio di nome GitHub reindirizza, valgono entrambi).
$top = (Run 'git rev-parse --show-toplevel' { git rev-parse --show-toplevel }) | Select-Object -First 1
if ((Resolve-Path $top).Path.TrimEnd('\') -ne (Resolve-Path $Root).Path.TrimEnd('\')) {
    Fail "lo script non è nella radice del repository Git ($top)"
}
$origin = (Run 'git remote get-url origin' { git remote get-url origin }) | Select-Object -First 1
if ($origin -notmatch 'tarabaz/(ForgeRender|RenderingEngine)(\.git)?/?$') {
    Fail "il remoto 'origin' ($origin) non è tarabaz/ForgeRender"
}
if (-not (Test-Path (Join-Path $Root 'crates\forge_app\Cargo.toml')) -or
    -not (Test-Path (Join-Path $Root 'packaging\windows\package.ps1'))) {
    Fail 'struttura del repository inattesa (manca crates\forge_app o packaging\windows\package.ps1)'
}

# Branch giusto (non si cambia branch da soli).
$current = (Run 'git branch --show-current' { git branch --show-current }) | Select-Object -First 1
if ($current -ne $Branch) {
    Fail "sei sul branch '$current', non su '$Branch'. Passa al branch giusto con:  git switch $Branch"
}

# Nessuna modifica locale ai file tracciati (i file non tracciati non bloccano: il pull li
# protegge da solo e si ferma se ne sovrascriverebbe uno).
$dirty = Run 'git status' { git status --porcelain --untracked-files=no }
if ($dirty) {
    Write-Host '    Modifiche locali non committate:' -ForegroundColor Yellow
    $dirty | ForEach-Object { Write-Host "      $_" -ForegroundColor Yellow }
    Fail 'ci sono modifiche locali non committate: non le tocco. Salvale con un commit o annullale tu, poi rilancia.'
}
Write-Host "    Repository: $Root"
Write-Host "    Remoto:     $origin"
Write-Host "    Branch:     $Branch (pulito)"

# ------------------------------------------------------------------ 2. Aggiornamento
Step "git pull --ff-only origin $Branch"
$before = (Run 'git rev-parse HEAD' { git rev-parse --short=7 HEAD }) | Select-Object -First 1
$pullOut = Run 'git pull --ff-only (aggiornamento non riuscito: niente build di una versione forse vecchia)' {
    git pull --ff-only origin $Branch
}
$after = (Run 'git rev-parse HEAD' { git rev-parse --short=7 HEAD }) | Select-Object -First 1
$pullOut | ForEach-Object { Write-Host "    $_" }
if ($before -eq $after) {
    $pullSummary = "già aggiornato, commit $after"
} else {
    $news = Run 'git log' { git log --oneline "$before..$after" }
    $pullSummary = "aggiornato da $before a $after ($(@($news).Count) commit nuovi)"
    Write-Host '    Commit nuovi:'
    $news | ForEach-Object { Write-Host "      $_" }
    $scriptChanged = Run 'git diff' { git diff --name-only $before $after -- Build-ForgeRender.ps1 }
    if ($scriptChanged) {
        Write-Host '    Nota: questo script è stato aggiornato; la nuova versione vale dal prossimo lancio.' -ForegroundColor Yellow
    }
}

# ------------------------------------------------------------------ 3. Compilazione
$version = (Select-String -Path (Join-Path $Root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
Step "cargo build --release -p forge_app -p forge_cli -p forge_mcp_bridge  (ForgeRender v$version, commit $after)"
# Output di cargo a schermo in diretta (la compilazione può durare minuti).
cargo build --release -p forge_app -p forge_cli -p forge_mcp_bridge
if ($LASTEXITCODE -ne 0) { Fail "compilazione fallita (codice di uscita $LASTEXITCODE)" }
$bin = Join-Path $Root 'target\release'
foreach ($exe in 'forgerender.exe', 'forgerender-cli.exe', 'forgerender-mcp.exe') {
    if (-not (Test-Path (Join-Path $bin $exe))) { Fail "dopo la compilazione manca target\release\$exe" }
}

# ------------------------------------------------------------------ 4. Convertitore STEP
# Facoltativo: se non si compila, ForgeRender funziona lo stesso ma non importa STEP/SolidWorks.
$stepExe = Join-Path $bin 'forgerender-step.exe'
$stepSummary = 'non incluso (-SkipStep)'
if (-not $SkipStep) {
    # CMake installato ma non nel PATH (installazione appena fatta, o quello di Visual Studio):
    # lo si aggiunge al PATH solo per questo script.
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        $cmakeCandidates = @(
            "$env:ProgramFiles\CMake\bin\cmake.exe",
            "${env:ProgramFiles(x86)}\CMake\bin\cmake.exe"
        ) + @(Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages", "$env:ProgramFiles\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio" `
                -Filter cmake.exe -Recurse -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -like '*\bin\cmake.exe' } | ForEach-Object { $_.FullName })
        $found = $cmakeCandidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
        if ($found) {
            $env:PATH = (Split-Path $found -Parent) + ';' + $env:PATH
            Write-Host "    CMake trovato in $found"
        }
    }
    # winget installa CMake "portable" e lo raggiunge con un collegamento: avviato da lì CMake
    # può non trovare i suoi moduli (CMAKE_ROOT). Si mette davanti la cartella vera.
    $cmakeCmd = Get-Command cmake -ErrorAction SilentlyContinue
    if ($cmakeCmd) {
        $item = Get-Item $cmakeCmd.Source -ErrorAction SilentlyContinue
        if ($item -and $item.LinkType -and $item.Target) {
            $real = @($item.Target)[0]
            if (-not [System.IO.Path]::IsPathRooted($real)) { $real = Join-Path (Split-Path $item.FullName -Parent) $real }
            if (Test-Path $real) {
                $env:PATH = (Split-Path $real -Parent) + ';' + $env:PATH
                Write-Host "    CMake (collegamento di winget) in $real"
            }
        }
    }
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        $stepSummary = 'NON incluso: manca CMake. Installalo con  winget install Kitware.CMake  poi riapri PowerShell e rilancia lo script'
        Write-Host "`n    $stepSummary" -ForegroundColor Yellow
    } else {
        Step 'cargo build --release -p forge_step  (convertitore STEP: la prima volta decine di minuti)'
        cargo build --release -p forge_step
        if ($LASTEXITCODE -eq 0 -and (Test-Path $stepExe)) {
            $stepSummary = "$stepExe"
        } else {
            $stepSummary = "NON incluso: compilazione fallita (codice $LASTEXITCODE), vedi i messaggi sopra"
            Write-Host "`n    $stepSummary" -ForegroundColor Yellow
        }
    }
}
$withStep = $stepSummary -eq "$stepExe"

# ------------------------------------------------------------------ 5. Pacchetto
Step 'Pacchetto portable (packaging\windows\package.ps1 -SkipBuild)'
$pkg = Join-Path $Root 'packaging\windows\package.ps1'
# Processo PowerShell separato: il pacchetto non eredita le impostazioni di questo script e
# l'esito si legge dal codice di uscita. Bypass vale solo per quel processo.
$pkgArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $pkg, '-SkipBuild')
if ($SkipInstaller) { $pkgArgs += '-SkipInstaller' }
if ($withStep) { $pkgArgs += '-WithStep' }
& powershell.exe @pkgArgs
if ($LASTEXITCODE -ne 0) { Fail "creazione del pacchetto fallita (codice di uscita $LASTEXITCODE)" }
$dist = Join-Path $Root 'dist'
$zip = Join-Path $dist "ForgeRender-$version-portable-x64.zip"
if (-not (Test-Path $zip) -or (Get-Item $zip).LastWriteTime -lt $started) {
    Fail "il pacchetto portable $zip non è stato creato da questa esecuzione"
}
# Denoiser anche accanto a target\release\forgerender.exe: percorso fisso per Claude Code (MCP),
# che non cambia a ogni versione come la cartella del portable (docs/MCP.md).
$portableDir = Join-Path $dist "ForgeRender-$version-portable-x64"
Get-ChildItem -Path $portableDir -Filter '*.dll' -ErrorAction SilentlyContinue | Copy-Item -Destination $bin -Force
$setup = Join-Path $dist "ForgeRender-$version-setup-x64.exe"
$setupNew = (Test-Path $setup) -and (Get-Item $setup).LastWriteTime -ge $started

# ------------------------------------------------------------------ Riepilogo
$elapsed = (Get-Date) - $started
Write-Host "`n================ ForgeRender v$version ($after) ================" -ForegroundColor Green
Write-Host "Pull:        $pullSummary"
Write-Host 'Build:       riuscita'
Write-Host "Eseguibili:  $bin\forgerender.exe"
Write-Host "             $bin\forgerender-cli.exe"
Write-Host "             $bin\forgerender-mcp.exe  (ponte per Claude, docs\MCP.md)"
Write-Host "STEP:        $stepSummary"
Write-Host "Portable:    $zip"
Write-Host "Cartella:    $(Join-Path $dist "ForgeRender-$version-portable-x64")"
if ($setupNew) { Write-Host "Installer:   $setup" }
Write-Host ('Tempo:       {0:mm\:ss}' -f $elapsed)
if (-not $NoOpen) { Invoke-Item $dist }
