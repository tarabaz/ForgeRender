//! Ponte verso SolidWorks (Windows, SolidWorks installato): il documento
//! SLDPRT/SLDASM viene esportato da SolidWorks stesso tramite le sue API COM.
//!
//! Formato preferito: **GLB** ("Extended Reality" di SolidWorks), che porta
//! apparenze come materiali PBR, texture, nomi e gerarchia dell'assieme, e si
//! importa senza il convertitore STEP. Lo STEP resta come ripiego.

use crate::blender::BlendExport;
use crate::ImportError;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn temp_path(ext: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("forgerender_solidworks_{}_{nanos}.{ext}", std::process::id()))
}

fn hide_console(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Esporta in GLB tramite SolidWorks. Il file temporaneo va rimosso dal chiamante.
pub fn export_glb(input: &Path) -> Result<BlendExport, ImportError> {
    let out = temp_path("glb");
    export(input, &out)?;
    Ok(BlendExport {
        glb: out,
        area_lights: Vec::new(),
    })
}

/// Script PowerShell che usa le API COM ufficiali di SolidWorks: apre il documento
/// in sola lettura e senza finestre di dialogo, lo esporta nel formato indicato dall'estensione di `$Out` (GLB o STEP) e lo chiude.
/// Se SolidWorks era già aperto viene riutilizzato, altrimenti viene avviato e poi chiuso.
const SOLIDWORKS_SCRIPT: &str = r#"
param([string]$In, [string]$Out)
$ErrorActionPreference = "Stop"
$started = $false
try { $sw = [Runtime.InteropServices.Marshal]::GetActiveObject("SldWorks.Application") }
catch {
  try { $sw = New-Object -ComObject "SldWorks.Application"; $started = $true }
  catch { [Console]::Error.WriteLine("SolidWorks non è installato o non è registrato per l'automazione COM"); exit 3 }
}
try {
  $ext = [IO.Path]::GetExtension($In).ToLower()
  $type = if ($ext -eq ".sldasm") { 2 } else { 1 }   # swDocASSEMBLY = 2, swDocPART = 1
  $err = 0; $warn = 0
  # swOpenDocOptions_Silent (1) | swOpenDocOptions_ReadOnly (2)
  try { $doc = $sw.OpenDoc6($In, $type, 3, "", [ref]$err, [ref]$warn) } catch { $doc = $sw.OpenDoc($In, $type) }
  if ($null -eq $doc) { [Console]::Error.WriteLine("SolidWorks non ha aperto il documento (codice $err)"); exit 4 }
  if ($type -eq 2) { try { [void]$doc.ResolveAllLightWeightComponents($false) } catch {} }
  # swSaveAsCurrentVersion (0), swSaveAsOptions_Silent (1) | swSaveAsOptions_Copy (2): il documento originale non viene modificato
  $res = $doc.SaveAs3($Out, 0, 3)
  $title = $doc.GetTitle()
  try { $sw.CloseDoc($title) } catch {}
  if (-not (Test-Path $Out)) { [Console]::Error.WriteLine("esportazione da SolidWorks fallita (codice $res)"); exit 5 }
} finally {
  if ($started) { try { $sw.ExitApp() } catch {} }
}
exit 0
"#;

/// Esporta un documento SolidWorks in `out` (il formato segue l'estensione: `.glb`, `.step`).
pub fn export(input: &Path, out: &Path) -> Result<(), ImportError> {
    if !cfg!(windows) {
        return Err(ImportError::Converter(
            "i file SolidWorks richiedono Windows con SolidWorks installato".into(),
        ));
    }
    let script = temp_path("ps1");
    std::fs::write(&script, SOLIDWORKS_SCRIPT)?;
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg("-In")
        .arg(input)
        .arg("-Out")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let result = hide_console(&mut cmd).output();
    let _ = std::fs::remove_file(&script);
    let output = result.map_err(|e| ImportError::Converter(format!("avvio di PowerShell fallito: {e}")))?;
    if !output.status.success() || !out.is_file() {
        let msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ImportError::Converter(if msg.is_empty() {
            "esportazione da SolidWorks fallita".into()
        } else {
            msg
        }));
    }
    Ok(())
}
