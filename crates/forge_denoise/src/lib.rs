//! Riduzione del rumore con Intel Open Image Denoise (OIDN 2.x).
//!
//! La libreria è caricata a runtime (nessun collegamento in fase di build):
//! se `OpenImageDenoise.dll` / `libOpenImageDenoise.so.2` non si trova, il
//! denoiser semplicemente non è disponibile e il resto dell'applicazione
//! funziona uguale. Ricerca, in ordine: `FORGE_OIDN_DIR`, la cartella
//! dell'eseguibile, la sottocartella `oidn`, i percorsi di sistema.
//!
//! Dispositivi: CPU (sempre) e CUDA per le GPU NVIDIA supportate da OIDN
//! (Volta e successive, incluse le RTX serie 50; non le Pascal come le Quadro
//! P-series). Se CUDA non è disponibile si ripiega sulla CPU, segnalandolo.
//!
//! I dati passano da buffer OIDN (`oidnNewBuffer` + lettura/scrittura), che
//! funzionano con qualsiasi dispositivo, anche quando la memoria della GPU
//! non è accessibile dall'host.

mod ffi;

use anyhow::{anyhow, bail, Context};
use ffi::{Api, OidnBuffer, OidnDevice};
pub use forge_render_core::Denoiser;
use forge_render_core::{DenoiseQuality, Guides, RenderScene, RenderSettings};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

const DEVICE_CPU: i32 = 1;
const DEVICE_CUDA: i32 = 3;
const FORMAT_FLOAT3: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Cpu,
    Cuda,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Cpu => "CPU",
            Kind::Cuda => "CUDA",
        }
    }
}

struct Device {
    kind: Kind,
    raw: OidnDevice,
}

pub struct OidnDenoiser {
    api: Api,
    path: PathBuf,
    cpu: Option<Device>,
    cuda: Option<Device>,
    /// Errore della creazione del dispositivo CUDA (non si riprova a ogni immagine).
    cuda_error: Option<String>,
    last: String,
}

// SAFETY: i dispositivi OIDN sono usati da un solo thread alla volta (la sessione di
// rendering possiede il denoiser); OIDN consente di usarli da thread diversi in sequenza.
unsafe impl Send for OidnDenoiser {}

impl OidnDenoiser {
    /// Carica la libreria e crea il dispositivo CPU (che serve sempre come ripiego).
    pub fn load() -> anyhow::Result<Self> {
        let mut errors = Vec::new();
        for path in candidates() {
            // SAFETY: si carica la libreria ufficiale di OIDN; i simboli sono verificati in `Api::load`.
            match unsafe { Api::load(&path) } {
                Ok(api) => {
                    let mut d = Self {
                        api,
                        path: path.clone(),
                        cpu: None,
                        cuda: None,
                        cuda_error: None,
                        last: String::new(),
                    };
                    d.cpu = Some(d.new_device(Kind::Cpu).context("creazione del dispositivo CPU di OIDN")?);
                    tracing::info!("Open Image Denoise {} caricato da {}", d.version(), path.display());
                    return Ok(d);
                }
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        bail!("Open Image Denoise non trovato ({})", errors.join("; "))
    }

    /// Versione della libreria, es. "2.3.3".
    pub fn version(&self) -> String {
        let Some(d) = &self.cpu else { return "?".into() };
        let name = CString::new("version").unwrap();
        // SAFETY: dispositivo valido, nome terminato.
        let v = unsafe { (self.api.get_device_int)(d.raw, name.as_ptr()) };
        format!("{}.{}.{}", v / 10000, (v / 100) % 100, v % 100)
    }

    pub fn library_path(&self) -> &Path {
        &self.path
    }

    /// Vero se il dispositivo CUDA è utilizzabile (lo crea al primo controllo).
    pub fn cuda_available(&mut self) -> bool {
        self.device(true).map(|d| d.kind == Kind::Cuda).unwrap_or(false)
    }

    pub fn cuda_error(&self) -> Option<&str> {
        self.cuda_error.as_deref()
    }

    fn take_error(&self, device: OidnDevice) -> Option<String> {
        let mut msg: *const std::ffi::c_char = std::ptr::null();
        // SAFETY: `device` può essere nullo (errori globali); `msg` punta a una stringa di OIDN.
        let code = unsafe { (self.api.get_device_error)(device, &mut msg) };
        if code == 0 {
            return None;
        }
        let text = if msg.is_null() {
            String::new()
        } else {
            // SAFETY: stringa terminata posseduta da OIDN, valida fino alla prossima chiamata.
            unsafe { CStr::from_ptr(msg) }.to_string_lossy().into_owned()
        };
        Some(format!("codice {code}: {text}"))
    }

    fn new_device(&self, kind: Kind) -> anyhow::Result<Device> {
        let ty = match kind {
            Kind::Cpu => DEVICE_CPU,
            Kind::Cuda => DEVICE_CUDA,
        };
        // SAFETY: chiamate FFI con argomenti validi; il dispositivo è rilasciato in `Drop`.
        let raw = unsafe { (self.api.new_device)(ty) };
        if raw.is_null() {
            bail!(
                "dispositivo {} non disponibile{}",
                kind.label(),
                self.take_error(std::ptr::null_mut()).map(|e| format!(" ({e})")).unwrap_or_default()
            );
        }
        unsafe { (self.api.commit_device)(raw) };
        if let Some(e) = self.take_error(raw) {
            unsafe { (self.api.release_device)(raw) };
            bail!("dispositivo {}: {e}", kind.label());
        }
        Ok(Device { kind, raw })
    }

    fn device(&mut self, use_cuda: bool) -> anyhow::Result<&Device> {
        if use_cuda && self.cuda.is_none() && self.cuda_error.is_none() {
            match self.new_device(Kind::Cuda) {
                Ok(d) => {
                    tracing::info!("OIDN: dispositivo CUDA attivo");
                    self.cuda = Some(d);
                }
                Err(e) => {
                    tracing::warn!("OIDN: CUDA non disponibile, si usa la CPU ({e:#})");
                    self.cuda_error = Some(format!("{e:#}"));
                }
            }
        }
        match (use_cuda, &self.cuda, &self.cpu) {
            (true, Some(d), _) => Ok(d),
            (_, _, Some(d)) => Ok(d),
            _ => Err(anyhow!("nessun dispositivo OIDN")),
        }
    }

    /// Filtro "RT" su colore HDR con guide opzionali.
    pub fn run(
        &mut self,
        color: &[[f32; 3]],
        guides: Option<&Guides>,
        width: u32,
        height: u32,
        quality: DenoiseQuality,
        use_cuda: bool,
    ) -> anyhow::Result<Vec<[f32; 3]>> {
        let n = width as usize * height as usize;
        anyhow::ensure!(color.len() == n, "immagine di dimensioni errate");
        if let Some(g) = guides {
            anyhow::ensure!(g.albedo.len() == n && g.normal.len() == n, "buffer guida di dimensioni errate");
        }
        let (raw, kind) = {
            let d = self.device(use_cuda)?;
            (d.raw, d.kind)
        };
        let api = &self.api;
        let bytes = n * 12;
        let q = match quality {
            DenoiseQuality::Fast => 4,
            DenoiseQuality::Balanced => 5,
            DenoiseQuality::High => 6,
        };
        let s = |t: &str| CString::new(t).unwrap();
        // SAFETY: tutte le chiamate OIDN usano oggetti creati qui e rilasciati prima di uscire;
        // i puntatori host restano validi per la durata di ogni copia sincrona.
        unsafe {
            let mut buffers: Vec<OidnBuffer> = Vec::new();
            let mut new_buf = |data: Option<&[[f32; 3]]>| -> anyhow::Result<OidnBuffer> {
                let b = (api.new_buffer)(raw, bytes);
                if b.is_null() {
                    bail!("allocazione del buffer OIDN fallita ({} MB)", bytes / (1 << 20));
                }
                if let Some(d) = data {
                    (api.write_buffer)(b, 0, bytes, d.as_ptr() as *const std::ffi::c_void);
                }
                buffers.push(b);
                Ok(b)
            };
            let result = (|| -> anyhow::Result<Vec<[f32; 3]>> {
                let c = new_buf(Some(color))?;
                let out = new_buf(None)?;
                let aux = match guides {
                    Some(g) => Some((new_buf(Some(&g.albedo))?, new_buf(Some(&g.normal))?)),
                    None => None,
                };
                let filter = (api.new_filter)(raw, s("RT").as_ptr());
                if filter.is_null() {
                    bail!("filtro RT non creato: {}", self.take_error(raw).unwrap_or_default());
                }
                let (w, h) = (width as usize, height as usize);
                (api.set_filter_image)(filter, s("color").as_ptr(), c, FORMAT_FLOAT3, w, h, 0, 0, 0);
                (api.set_filter_image)(filter, s("output").as_ptr(), out, FORMAT_FLOAT3, w, h, 0, 0, 0);
                if let Some((a, nrm)) = aux {
                    (api.set_filter_image)(filter, s("albedo").as_ptr(), a, FORMAT_FLOAT3, w, h, 0, 0, 0);
                    (api.set_filter_image)(filter, s("normal").as_ptr(), nrm, FORMAT_FLOAT3, w, h, 0, 0, 0);
                }
                (api.set_filter_bool)(filter, s("hdr").as_ptr(), true);
                (api.set_filter_int)(filter, s("quality").as_ptr(), q);
                (api.commit_filter)(filter);
                let err = self.take_error(raw);
                if err.is_none() {
                    (api.execute_filter)(filter);
                }
                let err = err.or_else(|| self.take_error(raw));
                (api.release_filter)(filter);
                if let Some(e) = err {
                    bail!("OIDN ({}): {e}", kind.label());
                }
                let mut img = vec![[0.0f32; 3]; n];
                (api.read_buffer)(out, 0, bytes, img.as_mut_ptr() as *mut std::ffi::c_void);
                if let Some(e) = self.take_error(raw) {
                    bail!("lettura del risultato OIDN: {e}");
                }
                Ok(img)
            })();
            for b in buffers {
                (api.release_buffer)(b);
            }
            let img = result?;
            self.last = format!("OIDN {} · {}", self.version(), kind.label());
            Ok(img)
        }
    }
}

impl Drop for OidnDenoiser {
    fn drop(&mut self) {
        for d in [self.cuda.take(), self.cpu.take()].into_iter().flatten() {
            // SAFETY: dispositivo creato da questa istanza, non più usato.
            unsafe { (self.api.release_device)(d.raw) };
        }
    }
}

impl Denoiser for OidnDenoiser {
    fn name(&self) -> String {
        if self.last.is_empty() {
            format!("OIDN {}", self.version())
        } else {
            self.last.clone()
        }
    }

    fn guides(&mut self, scene: &RenderScene, settings: &RenderSettings, cancel: &AtomicBool) -> Option<Guides> {
        forge_render_cpu::render_guides(scene, settings, forge_render_cpu::GUIDE_SAMPLES, cancel)
    }

    fn denoise(
        &mut self,
        color: &[[f32; 3]],
        guides: Option<&Guides>,
        width: u32,
        height: u32,
        quality: DenoiseQuality,
        use_cuda: bool,
    ) -> anyhow::Result<Vec<[f32; 3]>> {
        self.run(color, guides, width, height, quality, use_cuda)
    }
}

fn library_name() -> &'static str {
    if cfg!(windows) {
        "OpenImageDenoise.dll"
    } else if cfg!(target_os = "macos") {
        "libOpenImageDenoise.2.dylib"
    } else {
        "libOpenImageDenoise.so.2"
    }
}

fn candidates() -> Vec<PathBuf> {
    let name = library_name();
    let mut v = Vec::new();
    if let Some(dir) = std::env::var_os("FORGE_OIDN_DIR") {
        let dir = PathBuf::from(dir);
        v.push(dir.join(name));
        v.push(dir.join("lib").join(name));
        v.push(dir.join("bin").join(name));
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        v.push(exe_dir.join(name));
        v.push(exe_dir.join("oidn").join(name));
    }
    // Ricerca di sistema (PATH / LD_LIBRARY_PATH).
    v.push(PathBuf::from(name));
    v
}
