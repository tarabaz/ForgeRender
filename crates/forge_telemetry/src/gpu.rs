//! Utilizzo della GPU e memoria video (VRAM) tramite NVML, la libreria di misura del driver
//! NVIDIA (`nvml.dll` su Windows, `libnvidia-ml.so.1` su Linux). Caricata a runtime: senza driver
//! NVIDIA il programma funziona lo stesso e le statistiche dicono che la lettura non c'è.

use libloading::Library;
use std::ffi::{c_char, c_void};

type Device = *mut c_void;

#[repr(C)]
#[derive(Default)]
struct Utilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
#[derive(Default)]
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

struct Api {
    count: unsafe extern "C" fn(*mut u32) -> i32,
    handle: unsafe extern "C" fn(u32, *mut Device) -> i32,
    name: unsafe extern "C" fn(Device, *mut c_char, u32) -> i32,
    utilization: unsafe extern "C" fn(Device, *mut Utilization) -> i32,
    memory: unsafe extern "C" fn(Device, *mut Memory) -> i32,
    shutdown: unsafe extern "C" fn() -> i32,
    _lib: Library,
}

/// Una scheda NVIDIA all'ultima lettura.
#[derive(Clone, Debug, Default)]
pub struct GpuSample {
    pub name: String,
    /// Percentuale di tempo in cui la GPU ha lavorato nell'ultimo intervallo del driver.
    pub usage_percent: Option<u32>,
    pub vram_total_bytes: u64,
    pub vram_used_bytes: u64,
    pub vram_free_bytes: u64,
}

/// Lettore NVML. `None` da [`NvmlMonitor::open`] = nessun driver NVIDIA (o versione troppo vecchia).
pub struct NvmlMonitor {
    api: Api,
    devices: Vec<(Device, String)>,
}

// Le maniglie dei dispositivi NVML valgono per tutto il processo e la libreria è thread-safe.
unsafe impl Send for NvmlMonitor {}

#[cfg(windows)]
const CANDIDATES: &[&str] = &[
    "nvml.dll",
    // Driver più vecchi: nvml.dll non è in System32.
    r"C:\Program Files\NVIDIA Corporation\NVSMI\nvml.dll",
];
#[cfg(not(windows))]
const CANDIDATES: &[&str] = &["libnvidia-ml.so.1", "libnvidia-ml.so"];

impl NvmlMonitor {
    pub fn open() -> Option<Self> {
        let lib = CANDIDATES.iter().find_map(|p| unsafe { Library::new(p).ok() })?;
        // SAFETY: firme dell'API NVML (nvml.h), stabili dalle versioni _v2 (driver 2015 e successivi).
        let api = unsafe {
            macro_rules! sym {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes()).ok()?
                };
            }
            let init: unsafe extern "C" fn() -> i32 = sym!("nvmlInit_v2");
            if init() != 0 {
                return None;
            }
            Api {
                count: sym!("nvmlDeviceGetCount_v2"),
                handle: sym!("nvmlDeviceGetHandleByIndex_v2"),
                name: sym!("nvmlDeviceGetName"),
                utilization: sym!("nvmlDeviceGetUtilizationRates"),
                memory: sym!("nvmlDeviceGetMemoryInfo"),
                shutdown: sym!("nvmlShutdown"),
                _lib: lib,
            }
        };
        let mut n = 0u32;
        // SAFETY: puntatori validi a variabili locali, buffer del nome di 96 byte dichiarato.
        let devices = unsafe {
            if (api.count)(&mut n) != 0 {
                (api.shutdown)();
                return None;
            }
            (0..n)
                .filter_map(|i| {
                    let mut d: Device = std::ptr::null_mut();
                    if (api.handle)(i, &mut d) != 0 {
                        return None;
                    }
                    let mut buf = [0 as c_char; 96];
                    let name = if (api.name)(d, buf.as_mut_ptr(), buf.len() as u32) == 0 {
                        std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
                    } else {
                        format!("GPU NVIDIA {i}")
                    };
                    Some((d, name))
                })
                .collect::<Vec<_>>()
        };
        if devices.is_empty() {
            unsafe { (api.shutdown)() };
            return None;
        }
        Some(Self { api, devices })
    }

    /// Legge tutte le schede NVIDIA (pochi millisecondi).
    pub fn sample(&self) -> Vec<GpuSample> {
        self.devices
            .iter()
            .map(|(d, name)| {
                let mut u = Utilization::default();
                let mut m = Memory::default();
                // SAFETY: maniglie ottenute da NVML ancora inizializzata, strutture #[repr(C)].
                let (ru, rm) = unsafe { ((self.api.utilization)(*d, &mut u), (self.api.memory)(*d, &mut m)) };
                let mem_ok = rm == 0;
                GpuSample {
                    name: name.clone(),
                    usage_percent: (ru == 0).then_some(u.gpu),
                    vram_total_bytes: if mem_ok { m.total } else { 0 },
                    vram_used_bytes: if mem_ok { m.used } else { 0 },
                    vram_free_bytes: if mem_ok { m.free } else { 0 },
                }
            })
            .collect()
    }
}

impl Drop for NvmlMonitor {
    fn drop(&mut self) {
        unsafe { (self.api.shutdown)() };
    }
}

/// La scheda NVIDIA che corrisponde al nome del dispositivo di render (wgpu), se c'è.
/// Con un nome vuoto (render su CPU o GPU non ancora pronta) restituisce la prima.
pub fn matching<'a>(samples: &'a [GpuSample], device: &str) -> Option<&'a GpuSample> {
    if device.trim().is_empty() {
        return samples.first();
    }
    let dev = device.to_lowercase();
    samples.iter().find(|s| {
        let n = s.name.to_lowercase();
        dev.contains(&n) || n.contains(dev.split(" · ").next().unwrap_or(&dev).trim())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str) -> GpuSample {
        GpuSample {
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn picks_the_render_device() {
        let all = [s("NVIDIA GeForce RTX 3060"), s("NVIDIA RTX A2000")];
        assert_eq!(matching(&all, "").unwrap().name, "NVIDIA GeForce RTX 3060");
        let a2000 = matching(&all, "NVIDIA RTX A2000 · Vulkan · 4 campioni/passata").unwrap();
        assert_eq!(a2000.name, "NVIDIA RTX A2000");
        assert!(matching(&all, "AMD Radeon RX 7800 XT · Dx12 · 4 campioni/passata").is_none());
    }

    #[test]
    fn open_without_driver_is_harmless() {
        // Su macchine senza NVIDIA dà None; con NVIDIA le letture devono essere coerenti.
        if let Some(m) = NvmlMonitor::open() {
            for g in m.sample() {
                assert!(g.vram_used_bytes <= g.vram_total_bytes);
            }
        }
    }
}
