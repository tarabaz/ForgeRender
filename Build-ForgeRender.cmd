@echo off
rem Avvia Build-ForgeRender.ps1 anche se Windows blocca gli script PowerShell.
rem -ExecutionPolicy Bypass vale solo per questo processo: le impostazioni di Windows non cambiano.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Build-ForgeRender.ps1" %*
exit /b %ERRORLEVEL%
