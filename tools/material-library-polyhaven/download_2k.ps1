$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot 'download_polyhaven_materials.py'
$libraryPath = Join-Path $HOME 'Documents\ForgeRender\Assets\PolyHaven'
New-Item -ItemType Directory -Force -Path $libraryPath | Out-Null
Write-Host "Libreria destinazione: $libraryPath"
py -3 $scriptPath --output $libraryPath --resolution 2k --skip-existing
if ($LASTEXITCODE -ne 0) { throw "Download terminato con errori. Controlla errors.txt nella libreria." }
Write-Host 'Libreria Poly Haven aggiornata.'
