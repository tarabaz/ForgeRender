//! Video dai fotogrammi PNG di una sequenza: contenitore, codec e qualità li sceglie l'utente.
//!
//! Tre strade, e nessuna ne sostituisce un'altra di nascosto:
//! - H.264 in MP4 a bitrate fissato: Media Foundation di Windows (anche con l'encoder hardware
//!   della scheda video); se manca (edizioni "N") lo stesso H.264 con lo stesso bitrate da ffmpeg;
//! - AVI non compresso: scritto da qui, senza dipendenze (`avi`);
//! - tutto il resto (CRF, H.265, FFV1, ProRes, MOV, MKV): `ffmpeg`, se è accanto al programma o nel
//!   PATH. Senza ffmpeg queste scelte non compaiono.
//!
//! La codifica non tocca i fotogrammi già calcolati. I file nascono come `.tmp` e prendono il nome
//! finale solo a lavoro riuscito: un errore o un'interruzione non lasciano mai un video a metà.

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// Bitrate scelto per una buona qualità senza file enormi: ~0,2 bit per pixel per fotogramma
/// (1920 × 1080 a 30 fps ≈ 12 Mbit/s), tra 4 e 80 Mbit/s.
pub fn bitrate_for(width: u32, height: u32, fps: u32) -> u32 {
    bitrate_at(width, height, fps, STANDARD_BPP).min(80_000_000)
}

const STANDARD_BPP: f32 = 0.2;
const HIGH_BPP: f32 = 0.6;
const VERY_HIGH_BPP: f32 = 1.2;

fn bitrate_at(width: u32, height: u32, fps: u32, bpp: f32) -> u32 {
    let bits = width as f64 * height as f64 * fps.max(1) as f64 * bpp as f64;
    bits.clamp(4.0e6, 400.0e6) as u32
}

/// Contenitore: il tipo di file. Il codec è un'altra cosa (vedi [`Codec`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    Mp4,
    Mov,
    Mkv,
    Avi,
}

impl Container {
    pub const ALL: [Container; 4] = [Container::Mp4, Container::Mov, Container::Mkv, Container::Avi];

    pub fn label(self) -> &'static str {
        match self {
            Container::Mp4 => "MP4",
            Container::Mov => "MOV",
            Container::Mkv => "MKV",
            Container::Avi => "AVI",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mov => "mov",
            Container::Mkv => "mkv",
            Container::Avi => "avi",
        }
    }

    /// Codec che questo contenitore può portare.
    pub fn codecs(self) -> &'static [Codec] {
        match self {
            Container::Mp4 => &[Codec::H264, Codec::H265],
            Container::Mov => &[Codec::ProResHq, Codec::ProRes4444, Codec::H264, Codec::H265],
            Container::Mkv => &[Codec::Ffv1, Codec::H264, Codec::H265],
            Container::Avi => &[Codec::Raw, Codec::Ffv1],
        }
    }

    fn ffmpeg_format(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mov => "mov",
            Container::Mkv => "matroska",
            Container::Avi => "avi",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Codec {
    H264,
    H265,
    ProResHq,
    ProRes4444,
    Ffv1,
    /// Non compresso (RGB a 8 bit).
    Raw,
}

impl Codec {
    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::H265 => "H.265 (HEVC)",
            Codec::ProResHq => "ProRes 422 HQ",
            Codec::ProRes4444 => "ProRes 4444",
            Codec::Ffv1 => "FFV1",
            Codec::Raw => "RGB",
        }
    }

    /// Formati di pixel possibili; il primo è il predefinito.
    pub fn pixels(self) -> &'static [Pixel] {
        match self {
            Codec::H264 => &[Pixel::Yuv420, Pixel::Yuv444],
            Codec::H265 => &[Pixel::Yuv420, Pixel::Yuv420P10],
            Codec::ProResHq => &[Pixel::Yuv422P10],
            Codec::ProRes4444 => &[Pixel::Yuv444P10],
            Codec::Ffv1 => &[Pixel::Rgb, Pixel::Yuv444],
            Codec::Raw => &[Pixel::Bgr24],
        }
    }

    /// Si regola la compressione (CRF o bitrate); gli altri codec hanno qualità fissa.
    pub fn has_rate(self) -> bool {
        matches!(self, Codec::H264 | Codec::H265)
    }

    fn ffmpeg_encoder(self) -> Option<&'static str> {
        match self {
            Codec::H264 => Some("libx264"),
            Codec::H265 => Some("libx265"),
            Codec::ProResHq | Codec::ProRes4444 => Some("prores_ks"),
            Codec::Ffv1 => Some("ffv1"),
            Codec::Raw => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pixel {
    Yuv420,
    Yuv420P10,
    Yuv422P10,
    Yuv444,
    Yuv444P10,
    Rgb,
    Bgr24,
}

impl Pixel {
    pub fn label(self) -> &'static str {
        match self {
            Pixel::Yuv420 => "YUV 4:2:0, 8 bit",
            Pixel::Yuv420P10 => "YUV 4:2:0, 10 bit",
            Pixel::Yuv422P10 => "YUV 4:2:2, 10 bit",
            Pixel::Yuv444 => "YUV 4:4:4, 8 bit",
            Pixel::Yuv444P10 => "YUV 4:4:4, 10 bit",
            Pixel::Rgb | Pixel::Bgr24 => "RGB, 8 bit",
        }
    }

    fn ffmpeg(self) -> &'static str {
        match self {
            Pixel::Yuv420 => "yuv420p",
            Pixel::Yuv420P10 => "yuv420p10le",
            Pixel::Yuv422P10 => "yuv422p10le",
            Pixel::Yuv444 => "yuv444p",
            Pixel::Yuv444P10 => "yuv444p10le",
            Pixel::Rgb => "bgr0",
            Pixel::Bgr24 => "bgr24",
        }
    }

    fn is_yuv(self) -> bool {
        !matches!(self, Pixel::Rgb | Pixel::Bgr24)
    }
}

/// Come si regola la compressione di H.264 e H.265.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rate {
    /// Qualità costante: il file è grande quanto serve.
    Crf,
    /// Dimensione prevedibile: la qualità segue la scena.
    Bitrate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    Standard,
    High,
    VeryHigh,
    Lossless,
    Uncompressed,
    Editing,
    Custom,
}

impl Preset {
    pub const ALL: [Preset; 6] = [
        Preset::Standard,
        Preset::High,
        Preset::VeryHigh,
        Preset::Editing,
        Preset::Lossless,
        Preset::Uncompressed,
    ];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoSettings {
    pub preset: Preset,
    pub container: Container,
    pub codec: Codec,
    pub rate: Rate,
    /// 0 = senza perdita, 51 = pessimo. Valori sensati tra 12 e 23.
    pub crf: u8,
    /// Bit per pixel per fotogramma: il bitrate segue risoluzione e frequenza.
    pub bits_per_pixel: f32,
    pub pixel: Pixel,
    /// Fotogrammi al secondo del video (`None` = quelli della timeline).
    pub fps: Option<u32>,
    /// Risoluzione del video (`None` = quella del render).
    pub size: Option<[u32; 2]>,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            preset: Preset::Standard,
            container: Container::Mp4,
            codec: Codec::H264,
            rate: Rate::Bitrate,
            crf: 18,
            bits_per_pixel: STANDARD_BPP,
            pixel: Pixel::Yuv420,
            fps: None,
            size: None,
        }
    }
}

impl VideoSettings {
    /// Si può usare questo preset con ciò che c'è su questo computer.
    pub fn preset_available(p: Preset, caps: &Caps) -> bool {
        match p {
            Preset::Lossless => caps.has(Codec::Ffv1),
            Preset::Editing => caps.has(Codec::ProResHq),
            _ => true,
        }
    }

    /// Applica un preset: fps e risoluzione restano quelli scelti.
    pub fn apply_preset(&mut self, p: Preset, caps: &Caps) {
        let keep = (self.fps, self.size, self.crf, self.bits_per_pixel);
        let x264 = caps.has(Codec::H264);
        *self = match p {
            Preset::Standard | Preset::Custom => Self::default(),
            Preset::High | Preset::VeryHigh => Self {
                rate: if x264 { Rate::Crf } else { Rate::Bitrate },
                crf: if p == Preset::High { 16 } else { 12 },
                bits_per_pixel: if p == Preset::High { HIGH_BPP } else { VERY_HIGH_BPP },
                ..Self::default()
            },
            Preset::Lossless => Self {
                container: Container::Mkv,
                codec: Codec::Ffv1,
                pixel: Pixel::Rgb,
                ..Self::default()
            },
            Preset::Uncompressed => Self {
                container: Container::Avi,
                codec: Codec::Raw,
                pixel: Pixel::Bgr24,
                ..Self::default()
            },
            Preset::Editing => Self {
                container: Container::Mov,
                codec: Codec::ProResHq,
                pixel: Pixel::Yuv422P10,
                ..Self::default()
            },
        };
        self.preset = p;
        self.fps = keep.0;
        self.size = keep.1;
        if !matches!(p, Preset::High | Preset::VeryHigh) {
            self.crf = keep.2;
            self.bits_per_pixel = keep.3;
        }
        if p == Preset::Standard {
            self.bits_per_pixel = STANDARD_BPP;
        }
    }

    /// Codec che si possono scegliere ora, nel contenitore attuale.
    pub fn codecs(&self, caps: &Caps) -> Vec<Codec> {
        let c = self.container;
        c.codecs().iter().copied().filter(|k| caps.can(c, *k)).collect()
    }

    /// Dopo una modifica: riporta il resto su valori che il contenitore e il codec ammettono
    /// (si vede subito nella finestra, niente viene cambiato di nascosto alla codifica).
    pub fn sanitize(&mut self, caps: &Caps) {
        let codecs = self.codecs(caps);
        if !codecs.contains(&self.codec) {
            if let Some(c) = codecs.first() {
                self.codec = *c;
            }
        }
        if !self.codec.pixels().contains(&self.pixel) {
            self.pixel = self.codec.pixels()[0];
        }
        // Solo con Media Foundation non c'è il CRF.
        if self.rate == Rate::Crf && !caps.has(self.codec) {
            self.rate = Rate::Bitrate;
        }
        // Media Foundation vuole 4:2:0.
        if self.codec == Codec::H264 && self.pixel != Pixel::Yuv420 && !caps.has(Codec::H264) {
            self.pixel = Pixel::Yuv420;
        }
        self.crf = self.crf.min(51);
        self.bits_per_pixel = self.bits_per_pixel.clamp(0.01, 8.0);
    }

    /// Bitrate in bit al secondo, per i codec che lo usano.
    pub fn bitrate(&self, w: u32, h: u32, fps: u32) -> Option<u32> {
        (self.codec.has_rate() && self.rate == Rate::Bitrate).then(|| {
            let b = bitrate_at(w, h, fps, self.bits_per_pixel);
            if self.preset == Preset::Standard {
                b.min(bitrate_for(w, h, fps))
            } else {
                b
            }
        })
    }

    /// Lati del video da fotogrammi di `w × h`: i codec a colore ridotto vogliono lati pari.
    pub fn output_size(&self, w: u32, h: u32) -> (u32, u32) {
        let (w, h) = self.size.map_or((w, h), |s| (s[0], s[1]));
        if self.codec == Codec::Raw {
            (w, h)
        } else {
            (w & !1, h & !1)
        }
    }

    /// Stima della dimensione del file. `exact` solo per il non compresso.
    pub fn estimate(&self, w: u32, h: u32, fps: u32, frames: u64) -> Option<Estimate> {
        let (w, h) = self.output_size(w, h);
        let seconds = frames as f64 / fps.max(1) as f64;
        let pixels = w as f64 * h as f64;
        let by_bits = |bpp: f64| pixels * fps as f64 * bpp * seconds / 8.0;
        let (bytes, exact) = match self.codec {
            Codec::Raw => (avi::frame_bytes(w, h) as f64 * frames as f64, true),
            Codec::ProResHq => (by_bits(3.5), false),
            Codec::ProRes4444 => (by_bits(5.3), false),
            Codec::Ffv1 => return None,
            Codec::H264 | Codec::H265 => (self.bitrate(w, h, fps)? as f64 * seconds / 8.0, false),
        };
        Some(Estimate {
            bytes: bytes as u64,
            exact,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    pub bytes: u64,
    pub exact: bool,
}

/// Che cosa si può usare su questo computer.
#[derive(Clone, Debug, Default)]
pub struct Caps {
    pub ffmpeg: Option<PathBuf>,
    encoders: Vec<String>,
}

impl Caps {
    /// C'è l'encoder ffmpeg di questo codec.
    pub fn has(&self, codec: Codec) -> bool {
        self.ffmpeg.is_some() && codec.ffmpeg_encoder().is_some_and(|e| self.encoders.iter().any(|x| x == e))
    }

    /// Questa coppia contenitore e codec si può scrivere.
    pub fn can(&self, container: Container, codec: Codec) -> bool {
        if !container.codecs().contains(&codec) {
            return false;
        }
        match codec {
            Codec::Raw => true,
            Codec::H264 if container == Container::Mp4 && cfg!(windows) => true,
            _ => self.has(codec),
        }
    }
}

/// `ffmpeg` accanto al programma, altrimenti dal PATH.
fn find_ffmpeg() -> Option<PathBuf> {
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let beside = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(name)));
    let on_path = std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(name)).find(|f| f.is_file()));
    beside.filter(|p| p.is_file()).or(on_path)
}

fn no_console(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Si chiede a ffmpeg una volta sola quali encoder ha.
pub fn caps() -> &'static Caps {
    static CAPS: OnceLock<Caps> = OnceLock::new();
    CAPS.get_or_init(|| {
        let Some(ffmpeg) = find_ffmpeg() else { return Caps::default() };
        let mut cmd = std::process::Command::new(&ffmpeg);
        cmd.args(["-hide_banner", "-encoders"]);
        no_console(&mut cmd);
        let Ok(out) = cmd.output() else { return Caps::default() };
        // Righe come " V....D libx264    libx264 H.264 ...".
        let encoders = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
            .collect();
        Caps {
            ffmpeg: Some(ffmpeg),
            encoders,
        }
    })
}

/// Avanzamento della codifica, letto dalla barra di stato; `cancel` la interrompe.
#[derive(Default)]
pub struct Progress {
    pub done: AtomicUsize,
    pub total: AtomicUsize,
    pub cancel: AtomicBool,
}

impl Progress {
    pub fn get(&self) -> (usize, usize) {
        (self.done.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed))
    }

    fn tick(&self) -> anyhow::Result<()> {
        self.done.fetch_add(1, Ordering::Relaxed);
        self.check()
    }

    fn check(&self) -> anyhow::Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            bail!("interrotto");
        }
        Ok(())
    }
}

pub struct Job {
    pub frames: Vec<PathBuf>,
    /// Il file di destinazione: l'estensione è quella del contenitore.
    pub out: PathBuf,
    pub settings: VideoSettings,
    /// Fotogrammi al secondo se `settings.fps` non ne chiede altri.
    pub fps: u32,
    /// Giri del turntable: i fotogrammi si ripetono.
    pub loops: u32,
}

/// Quello che è stato scritto davvero.
#[derive(Clone, Debug)]
pub struct Report {
    pub path: PathBuf,
    pub encoder: &'static str,
    pub container: Container,
    pub codec: Codec,
    pub pixel: Pixel,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bytes: u64,
    pub frames: u64,
    /// Compressione: "CRF 16" oppure "12,4 Mbit/s".
    pub quality: String,
}

impl Report {
    /// Parametri effettivi, per il log e per l'avviso.
    pub fn describe(&self) -> String {
        let seconds = self.frames as f64 / self.fps.max(1) as f64;
        let mbit = self.bytes as f64 * 8.0 / seconds.max(1e-3) / 1e6;
        let quality = if self.quality.is_empty() {
            String::new()
        } else {
            format!(", {}", self.quality)
        };
        format!(
            "{} {}{quality}, {}, {}×{}, {} fps, {:.1} MB, {mbit:.1} Mbit/s ({})",
            self.container.label(),
            self.codec.label(),
            self.pixel.label(),
            self.width,
            self.height,
            self.fps,
            self.bytes as f64 / 1e6,
            self.encoder,
        )
    }
}

/// Stima del tempo che manca, dalla media di quanto già fatto. Con meno di due elementi finiti non
/// c'è una media credibile: meglio dirlo che mostrare un numero inventato.
pub fn eta(done: usize, total: usize, elapsed: std::time::Duration) -> Option<std::time::Duration> {
    if done < 2 || total < done {
        return None;
    }
    Some(elapsed.mul_f64((total - done) as f64 / done as f64))
}

/// Spazio libero sul disco di `path`.
#[cfg(windows)]
fn free_bytes(path: &Path) -> Option<u64> {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let dir = path.parent()?;
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(&HSTRING::from(dir.as_os_str()), Some(&mut free), None, None) }.ok()?;
    Some(free)
}

#[cfg(not(windows))]
fn free_bytes(_: &Path) -> Option<u64> {
    None
}

/// Scrive il video. Il file finale compare solo a lavoro riuscito.
pub fn encode(job: &Job, progress: &Progress) -> anyhow::Result<Report> {
    if job.frames.is_empty() {
        bail!("nessun fotogramma");
    }
    let s = job.settings.clone();
    let first = image::open(&job.frames[0]).with_context(|| format!("{}", job.frames[0].display()))?;
    let (w, h) = s.output_size(first.width(), first.height());
    if w < 16 || h < 16 {
        bail!("immagine troppo piccola per il video ({w} × {h})");
    }
    let fps = s.fps.unwrap_or(job.fps).max(1);
    let loops = job.loops.max(1);
    let total = job.frames.len() as u64 * loops as u64;
    progress.total.store(total as usize, Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);
    let out = job.out.with_extension(s.container.extension());
    if let (Some(e), Some(free)) = (s.estimate(first.width(), first.height(), fps, total), free_bytes(&out)) {
        if e.bytes > free {
            bail!(
                "spazio su disco insufficiente: servono circa {:.1} GB, liberi {:.1} GB",
                e.bytes as f64 / 1e9,
                free as f64 / 1e9
            );
        }
    }
    let tmp = out.with_extension(format!("{}.tmp", s.container.extension()));
    let _ = std::fs::remove_file(&tmp);
    let source = Source {
        frames: &job.frames,
        w,
        h,
        resize: s.size.is_some(),
        loops,
    };
    let result = write_with(&s, &source, &tmp, fps, progress);
    let encoder = match result {
        Ok(e) => e,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    };
    std::fs::rename(&tmp, &out).with_context(|| format!("{}", out.display()))?;
    let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    let bitrate = s.bitrate(w, h, fps);
    let quality = match (s.codec.has_rate(), bitrate) {
        (true, Some(b)) => format!("{:.1} Mbit/s", b as f64 / 1e6),
        (true, None) => format!("CRF {}", s.crf),
        _ => String::new(),
    };
    Ok(Report {
        path: out,
        encoder,
        container: s.container,
        codec: s.codec,
        pixel: s.pixel,
        width: w,
        height: h,
        fps,
        bytes,
        frames: total,
        quality,
    })
}

/// Sceglie la strada. H.264 a bitrate in MP4 è l'unico caso con due strade, e dànno lo stesso
/// codec con lo stesso bitrate.
fn write_with(s: &VideoSettings, src: &Source, tmp: &Path, fps: u32, progress: &Progress) -> anyhow::Result<&'static str> {
    let caps = caps();
    if s.codec == Codec::Raw {
        avi::write(src, tmp, fps, progress, avi::SEGMENT_BYTES)?;
        return Ok("ForgeRender");
    }
    #[cfg(windows)]
    if s.container == Container::Mp4 && s.codec == Codec::H264 && s.pixel == Pixel::Yuv420 {
        if let Some(bitrate) = s.bitrate(src.w, src.h, fps) {
            match media_foundation::encode(src, tmp, fps, bitrate, progress) {
                Ok(()) => return Ok("Media Foundation"),
                Err(e) => {
                    progress.check()?;
                    let _ = std::fs::remove_file(tmp);
                    progress.done.store(0, Ordering::Relaxed);
                    tracing::warn!("Media Foundation non disponibile per il video: {e:#}; provo ffmpeg");
                    if !caps.has(Codec::H264) {
                        return Err(e.context("encoder H.264 di Windows non disponibile (e ffmpeg non trovato)"));
                    }
                }
            }
        }
    }
    if !caps.has(s.codec) {
        bail!("{} richiede ffmpeg, che non è stato trovato", s.codec.label());
    }
    ffmpeg::encode(caps, s, src, tmp, fps, progress)?;
    Ok("ffmpeg")
}

/// I fotogrammi da codificare, alla misura finale.
struct Source<'a> {
    frames: &'a [PathBuf],
    w: u32,
    h: u32,
    resize: bool,
    loops: u32,
}

impl Source<'_> {
    /// Fotogramma come BGRA dall'alto, alla misura del video: ritagliato di una riga se serve un
    /// lato pari, ridimensionato (Lanczos) se è stata scelta un'altra risoluzione.
    fn bgra(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        let (w, h) = (self.w, self.h);
        let mut img = image::open(path).with_context(|| format!("{}", path.display()))?.to_rgba8();
        if self.resize {
            img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Lanczos3);
        }
        if img.width() < w || img.height() < h {
            bail!("{}: dimensioni diverse dagli altri fotogrammi", path.display());
        }
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let p = img.get_pixel(x, y).0;
                out.extend_from_slice(&[p[2], p[1], p[0], 255]);
            }
        }
        Ok(out)
    }
}

/// AVI non compresso (RGB a 8 bit) con gli indici OpenDML: oltre il gigabyte il file continua in
/// segmenti `AVIX`, così un video lungo non si ferma al limite dei 2 GB dell'AVI classico.
mod avi {
    use super::*;
    use std::io::{BufWriter, Seek, SeekFrom, Write};

    /// Un segmento RIFF per gigabyte circa.
    pub const SEGMENT_BYTES: u64 = 1 << 30;
    /// Voci dell'indice principale: con 1 GB l'una, 256 GB.
    const MAX_SEGMENTS: usize = 256;
    const STREAM: &[u8; 4] = b"00db";

    fn stride(w: u32) -> u64 {
        (w as u64 * 3).div_ceil(4) * 4
    }

    pub fn frame_bytes(w: u32, h: u32) -> u64 {
        stride(w) * h as u64
    }

    /// Scrive e tiene il conto della posizione.
    struct Out {
        w: BufWriter<std::fs::File>,
        pos: u64,
    }

    impl Out {
        fn put(&mut self, b: &[u8]) -> anyhow::Result<()> {
            self.w.write_all(b).context("scrittura del video (disco pieno?)")?;
            self.pos += b.len() as u64;
            Ok(())
        }

        fn u32(&mut self, v: u32) -> anyhow::Result<()> {
            self.put(&v.to_le_bytes())
        }

        fn u64(&mut self, v: u64) -> anyhow::Result<()> {
            self.put(&v.to_le_bytes())
        }

        fn u16(&mut self, v: u16) -> anyhow::Result<()> {
            self.put(&v.to_le_bytes())
        }

        /// Riscrive un valore più indietro nel file e torna dov'era.
        fn patch(&mut self, at: u64, b: &[u8]) -> anyhow::Result<()> {
            self.w.seek(SeekFrom::Start(at))?;
            self.w.write_all(b)?;
            self.w.seek(SeekFrom::Start(self.pos))?;
            Ok(())
        }

        /// Apre una lista: restituisce dove va scritta la sua dimensione.
        fn list(&mut self, kind: &[u8; 4], name: &[u8; 4]) -> anyhow::Result<u64> {
            self.put(kind)?;
            let at = self.pos;
            self.u32(0)?;
            self.put(name)?;
            Ok(at)
        }

        fn close(&mut self, at: u64) -> anyhow::Result<()> {
            let size = (self.pos - at - 4) as u32;
            self.patch(at, &size.to_le_bytes())
        }
    }

    /// Un segmento in scrittura: dove inizia e i fotogrammi che contiene.
    struct Segment {
        riff: u64,
        movi: u64,
        chunks: Vec<u64>,
    }

    fn close_segment(out: &mut Out, seg: &Segment, size: u32, first: bool, supers: &mut Vec<(u64, u32, u32)>) -> anyhow::Result<()> {
        let base = seg.movi + 4;
        // Indice del segmento, dentro `movi`.
        let at = out.pos;
        out.put(b"ix00")?;
        let bytes = 24 + 8 * seg.chunks.len() as u32;
        out.u32(bytes)?;
        out.u16(2)?;
        out.put(&[0, 1])?;
        out.u32(seg.chunks.len() as u32)?;
        out.put(STREAM)?;
        out.u64(base)?;
        out.u32(0)?;
        for c in &seg.chunks {
            out.u32((c - base) as u32)?;
            out.u32(size)?;
        }
        supers.push((at, bytes + 8, seg.chunks.len() as u32));
        out.close(seg.movi)?;
        // Il primo segmento ha anche l'indice classico, per i programmi che non leggono OpenDML.
        if first {
            out.put(b"idx1")?;
            out.u32(16 * seg.chunks.len() as u32)?;
            for c in &seg.chunks {
                out.put(STREAM)?;
                out.u32(0x10)?;
                out.u32((c - 8 - base) as u32)?;
                out.u32(size)?;
            }
        }
        out.close(seg.riff)
    }

    pub(super) fn write(src: &Source, path: &Path, fps: u32, progress: &Progress, segment: u64) -> anyhow::Result<()> {
        let (w, h) = (src.w, src.h);
        let frame = frame_bytes(w, h);
        if frame + 1024 > segment {
            bail!("fotogramma troppo grande per l'AVI ({w} × {h})");
        }
        let total = src.frames.len() as u64 * src.loops as u64;
        let file = std::fs::File::create(path).with_context(|| format!("{}", path.display()))?;
        let mut out = Out {
            w: BufWriter::with_capacity(1 << 20, file),
            pos: 0,
        };
        let riff = out.list(b"RIFF", b"AVI ")?;
        let hdrl = out.list(b"LIST", b"hdrl")?;
        out.put(b"avih")?;
        out.u32(56)?;
        out.u32(1_000_000 / fps)?;
        out.u32((frame * fps as u64).min(u32::MAX as u64) as u32)?;
        out.u32(0)?;
        out.u32(0x10)?; // ha l'indice
        let avih_frames = out.pos;
        out.u32(0)?;
        out.u32(0)?;
        out.u32(1)?;
        out.u32(frame as u32)?;
        out.u32(w)?;
        out.u32(h)?;
        out.put(&[0; 16])?;

        let strl = out.list(b"LIST", b"strl")?;
        out.put(b"strh")?;
        out.u32(56)?;
        out.put(b"vids")?;
        out.put(&[0; 4])?; // nessun codec: RGB
        out.u32(0)?;
        out.u32(0)?;
        out.u32(0)?;
        out.u32(1)?;
        out.u32(fps)?;
        out.u32(0)?;
        out.u32(total as u32)?;
        out.u32(frame as u32)?;
        out.u32(u32::MAX)?;
        out.u32(0)?;
        out.u16(0)?;
        out.u16(0)?;
        out.u16(w as u16)?;
        out.u16(h as u16)?;

        // Altezza positiva: le righe vanno scritte dal basso.
        out.put(b"strf")?;
        out.u32(40)?;
        out.u32(40)?;
        out.u32(w)?;
        out.u32(h)?;
        out.u16(1)?;
        out.u16(24)?;
        out.u32(0)?;
        out.u32(frame as u32)?;
        out.put(&[0; 16])?;

        // Indice principale: lo spazio si tiene ora, si riempie alla fine.
        out.put(b"indx")?;
        let indx_bytes = 24 + 16 * MAX_SEGMENTS as u32;
        out.u32(indx_bytes)?;
        let indx = out.pos;
        out.put(&vec![0; indx_bytes as usize])?;
        out.close(strl)?;

        let odml = out.list(b"LIST", b"odml")?;
        out.put(b"dmlh")?;
        out.u32(248)?;
        out.u32(total as u32)?;
        out.put(&[0; 244])?;
        out.close(odml)?;
        out.close(hdrl)?;

        let movi = out.list(b"LIST", b"movi")?;
        let mut seg = Segment {
            riff,
            movi,
            chunks: Vec::new(),
        };
        let mut supers: Vec<(u64, u32, u32)> = Vec::new();
        let mut first_frames = None;
        let row = stride(w) as usize;
        let mut buf = vec![0u8; frame as usize];
        let per_frame = frame + 8 + 16 + 8;
        for _ in 0..src.loops {
            for path in src.frames {
                // Segmento pieno: indice, chiusura e un nuovo `AVIX`.
                if out.pos - seg.riff + per_frame + 1024 > segment && !seg.chunks.is_empty() {
                    let first = first_frames.is_none();
                    close_segment(&mut out, &seg, frame as u32, first, &mut supers)?;
                    first_frames.get_or_insert(seg.chunks.len() as u32);
                    if supers.len() >= MAX_SEGMENTS {
                        bail!("video troppo lungo per l'AVI non compresso");
                    }
                    let riff = out.list(b"RIFF", b"AVIX")?;
                    let movi = out.list(b"LIST", b"movi")?;
                    seg = Segment {
                        riff,
                        movi,
                        chunks: Vec::new(),
                    };
                }
                let bgra = src.bgra(path)?;
                for y in 0..h as usize {
                    let from = &bgra[y * w as usize * 4..(y + 1) * w as usize * 4];
                    let to = &mut buf[(h as usize - 1 - y) * row..][..w as usize * 3];
                    for (d, s) in to.as_chunks_mut::<3>().0.iter_mut().zip(from.as_chunks::<4>().0) {
                        d.copy_from_slice(&s[..3]);
                    }
                }
                out.put(STREAM)?;
                out.u32(frame as u32)?;
                seg.chunks.push(out.pos);
                out.put(&buf)?;
                progress.tick()?;
            }
        }
        let first = first_frames.is_none();
        close_segment(&mut out, &seg, frame as u32, first, &mut supers)?;
        let first_frames = first_frames.unwrap_or(seg.chunks.len() as u32);
        out.patch(avih_frames, &first_frames.to_le_bytes())?;
        let mut idx = Vec::with_capacity(24 + 16 * supers.len());
        idx.extend_from_slice(&4u16.to_le_bytes());
        idx.extend_from_slice(&[0, 0]);
        idx.extend_from_slice(&(supers.len() as u32).to_le_bytes());
        idx.extend_from_slice(STREAM);
        idx.extend_from_slice(&[0; 12]);
        for (at, bytes, frames) in &supers {
            idx.extend_from_slice(&at.to_le_bytes());
            idx.extend_from_slice(&bytes.to_le_bytes());
            idx.extend_from_slice(&frames.to_le_bytes());
        }
        out.patch(indx, &idx)?;
        out.w.flush().context("chiusura del video (disco pieno?)")?;
        Ok(())
    }
}

#[cfg(windows)]
mod media_foundation {
    use super::*;
    use windows::core::HSTRING;
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

    /// Durata in unità da 100 ns di `frames` fotogrammi a `fps`.
    fn ticks(frames: u64, fps: u32) -> i64 {
        (frames * 10_000_000 / fps.max(1) as u64) as i64
    }

    pub(super) fn encode(src: &Source, out: &Path, fps: u32, bitrate: u32, progress: &Progress) -> anyhow::Result<()> {
        unsafe {
            // Il thread del video è nostro: COM multithread (S_FALSE se era già inizializzato).
            let com = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            let r = (|| -> anyhow::Result<()> {
                MFStartup(MF_VERSION, MFSTARTUP_FULL).context("MFStartup")?;
                let r = write(src, out, fps, bitrate, progress);
                let _ = MFShutdown();
                r
            })();
            if com {
                CoUninitialize();
            }
            r
        }
    }

    unsafe fn write(src: &Source, out: &Path, fps: u32, bitrate: u32, progress: &Progress) -> anyhow::Result<()> {
        let (w, h) = (src.w, src.h);
        unsafe {
            let mut attrs: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attrs, 2)?;
            let attrs = attrs.context("attributi")?;
            // Encoder della scheda video (NVENC, Quick Sync…) se il sistema lo offre.
            attrs.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1)?;
            // Il file ha estensione .tmp: il contenitore si dichiara esplicitamente.
            attrs.SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4)?;
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(out.as_os_str()), None, &attrs).context("creazione del file video")?;
            let size = ((w as u64) << 32) | h as u64;
            let rate = ((fps as u64) << 32) | 1;
            let square = (1u64 << 32) | 1;

            let out_type = MFCreateMediaType()?;
            out_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            out_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            out_type.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
            out_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            out_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
            out_type.SetUINT64(&MF_MT_FRAME_SIZE, size)?;
            out_type.SetUINT64(&MF_MT_FRAME_RATE, rate)?;
            out_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, square)?;
            let stream = writer.AddStream(&out_type).context("flusso H.264")?;

            let in_type = MFCreateMediaType()?;
            in_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            in_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            in_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            // Passo positivo: righe dall'alto verso il basso (come le PNG).
            in_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, w * 4)?;
            in_type.SetUINT64(&MF_MT_FRAME_SIZE, size)?;
            in_type.SetUINT64(&MF_MT_FRAME_RATE, rate)?;
            in_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, square)?;
            writer
                .SetInputMediaType(stream, &in_type, None)
                .context("conversione da RGB a H.264")?;
            writer.BeginWriting()?;

            let len = w * h * 4;
            let mut k = 0u64;
            for _ in 0..src.loops {
                for path in src.frames {
                    let bgra = src.bgra(path)?;
                    let buf = MFCreateMemoryBuffer(len)?;
                    let mut ptr = std::ptr::null_mut();
                    buf.Lock(&mut ptr, None, None)?;
                    std::ptr::copy_nonoverlapping(bgra.as_ptr(), ptr, len as usize);
                    buf.Unlock()?;
                    buf.SetCurrentLength(len)?;
                    let sample = MFCreateSample()?;
                    sample.AddBuffer(&buf)?;
                    sample.SetSampleTime(ticks(k, fps))?;
                    sample.SetSampleDuration(ticks(k + 1, fps) - ticks(k, fps))?;
                    writer.WriteSample(stream, &sample).context("scrittura di un fotogramma")?;
                    k += 1;
                    progress.tick()?;
                }
            }
            writer.Finalize().context("chiusura del video")?;
            Ok(())
        }
    }
}

mod ffmpeg {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Parametri di codifica per queste impostazioni.
    pub(super) fn codec_args(s: &VideoSettings, w: u32, h: u32, fps: u32) -> anyhow::Result<Vec<String>> {
        let encoder = s.codec.ffmpeg_encoder().context("codec senza encoder ffmpeg")?;
        let mut a: Vec<String> = vec!["-c:v".into(), encoder.into()];
        let mut add = |v: &[&str]| a.extend(v.iter().map(|x| x.to_string()));
        match s.codec {
            Codec::H264 => add(&["-preset", "slow"]),
            Codec::H265 => add(&["-preset", "medium", "-tag:v", "hvc1"]),
            Codec::ProResHq => add(&["-profile:v", "3", "-vendor", "apl0"]),
            Codec::ProRes4444 => add(&["-profile:v", "4", "-vendor", "apl0"]),
            Codec::Ffv1 => add(&["-level", "3", "-g", "1", "-slicecrc", "1"]),
            Codec::Raw => {}
        }
        if s.codec.has_rate() {
            match s.bitrate(w, h, fps) {
                Some(b) => add(&["-b:v", &b.to_string()]),
                None => add(&["-crf", &s.crf.to_string()]),
            }
        }
        // Dall'RGB dei fotogrammi allo YUV: conversione dichiarata e segnata nel file (BT.709).
        if s.pixel.is_yuv() {
            let vf = format!(
                "scale=in_range=pc:out_range=tv:out_color_matrix=bt709:flags=accurate_rnd+full_chroma_int,format={}",
                s.pixel.ffmpeg()
            );
            add(&[
                "-vf",
                &vf,
                "-colorspace",
                "bt709",
                "-color_primaries",
                "bt709",
                "-color_trc",
                "bt709",
            ]);
        }
        add(&["-pix_fmt", s.pixel.ffmpeg()]);
        if matches!(s.container, Container::Mp4 | Container::Mov) {
            add(&["-movflags", "+faststart"]);
        }
        add(&["-f", s.container.ffmpeg_format()]);
        Ok(a)
    }

    pub(super) fn encode(caps: &Caps, s: &VideoSettings, src: &Source, out: &Path, fps: u32, progress: &Progress) -> anyhow::Result<()> {
        let (w, h) = (src.w, src.h);
        let log = out.with_extension("log.tmp");
        let mut cmd = Command::new(caps.ffmpeg.as_ref().context("ffmpeg non trovato")?);
        cmd.args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "bgra", "-s"])
            .arg(format!("{w}x{h}"))
            .arg("-r")
            .arg(fps.to_string())
            .args(["-i", "-"])
            .args(codec_args(s, w, h, fps)?)
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log)?);
        no_console(&mut cmd);
        let mut child = cmd.spawn().context("ffmpeg non trovato")?;
        let written = (|| -> anyhow::Result<()> {
            let stdin = child.stdin.as_mut().context("ffmpeg")?;
            for _ in 0..src.loops {
                for path in src.frames {
                    stdin.write_all(&src.bgra(path)?).context("ffmpeg ha smesso di leggere")?;
                    progress.tick()?;
                }
            }
            Ok(())
        })();
        if let Err(e) = written {
            drop(child.stdin.take());
            let cancelled = progress.check().is_err();
            let _ = child.kill();
            let _ = child.wait();
            let detail = std::fs::read_to_string(&log).unwrap_or_default();
            let _ = std::fs::remove_file(&log);
            if !cancelled && !detail.trim().is_empty() {
                bail!("ffmpeg: {}", detail.trim());
            }
            return Err(e);
        }
        drop(child.stdin.take());
        let st = child.wait()?;
        let detail = std::fs::read_to_string(&log).unwrap_or_default();
        let _ = std::fs::remove_file(&log);
        if !st.success() {
            bail!("ffmpeg è terminato con errore ({st}): {}", detail.trim());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(dir: &Path, n: usize) -> Vec<PathBuf> {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();
        (0..n)
            .map(|i| {
                let c = [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]][i % 3];
                // Metà alta colorata, metà bassa bianca: il video non deve uscire capovolto.
                let img = image::RgbImage::from_fn(161, 91, |_, y| image::Rgb(if y < 45 { c } else { [255, 255, 255] }));
                let p = dir.join(format!("f{i}.png"));
                img.save(&p).unwrap();
                p
            })
            .collect()
    }

    fn job(frames: Vec<PathBuf>, out: PathBuf, settings: VideoSettings, loops: u32) -> Job {
        Job {
            frames,
            out,
            settings,
            fps: 30,
            loops,
        }
    }

    /// `(codec, pix_fmt, larghezza, altezza, frequenza, fotogrammi)` letti dal file, se c'è ffprobe.
    fn probe(path: &Path) -> Option<String> {
        let o = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-count_frames",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,pix_fmt,width,height,r_frame_rate,nb_read_frames",
                "-of",
                "csv=p=0",
            ])
            .arg(path)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    /// Primo fotogramma decodificato: in alto rosso, in basso bianco.
    fn check_first_frame(video: &Path) {
        let png = video.with_extension("primo.png");
        let done = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(video)
            .args(["-frames:v", "1"])
            .arg(&png)
            .status();
        if done.is_ok_and(|s| s.success()) {
            let f = image::open(&png).unwrap().to_rgb8();
            let (top, bottom) = (f.get_pixel(80, 10).0, f.get_pixel(80, 80).0);
            assert!(top[0] > 200 && top[1] < 60 && top[2] < 60, "alto {top:?}");
            assert!(bottom.iter().all(|&v| v > 200), "basso {bottom:?}");
        }
    }

    /// Il percorso standard: MP4 H.264 con i parametri dichiarati nel rapporto.
    #[test]
    fn frames_become_an_mp4() {
        let dir = std::env::temp_dir().join(format!("forge_video_{}", std::process::id()));
        let list = frames(&dir, 3);
        let out = dir.join("giro.mp4");
        let progress = Progress::default();
        let r = encode(&job(list, out.clone(), VideoSettings::default(), 2), &progress).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.len() > 500 && &bytes[4..8] == b"ftyp", "{}: {} byte", r.encoder, bytes.len());
        assert!(!out.with_extension("mp4.tmp").exists());
        if cfg!(windows) {
            assert_eq!(r.encoder, "Media Foundation");
        }
        assert_eq!((r.width, r.height, r.frames, r.bytes), (160, 90, 6, bytes.len() as u64));
        assert_eq!(progress.get(), (6, 6));
        check_first_frame(&out);
        if let Some(s) = probe(&out) {
            assert_eq!(s, "h264,160,90,yuv420p,30/1,6", "{}", r.encoder);
        }
        std::fs::remove_dir_all(dir).ok();
    }

    /// AVI non compresso scritto da noi, su più segmenti: lati dispari conservati, fotogrammi interi.
    #[test]
    fn uncompressed_avi_spans_segments() {
        let dir = std::env::temp_dir().join(format!("forge_video_avi_{}", std::process::id()));
        let list = frames(&dir, 3);
        let mut s = VideoSettings::default();
        s.apply_preset(Preset::Uncompressed, &Caps::default());
        assert_eq!((s.container, s.codec, s.pixel), (Container::Avi, Codec::Raw, Pixel::Bgr24));
        let src = Source {
            frames: &list,
            w: 161,
            h: 91,
            resize: false,
            loops: 4,
        };
        let out = dir.join("v.avi");
        let frame = avi::frame_bytes(161, 91);
        // Tre fotogrammi per segmento: 12 fotogrammi in un segmento classico e almeno tre AVIX.
        avi::write(&src, &out, 30, &Progress::default(), frame * 3 + 2048).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"AVI ");
        assert!(bytes.windows(4).filter(|w| w == b"AVIX").count() >= 3);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as u64 % 2, 0);
        check_first_frame(&out);
        if let Some(p) = probe(&out) {
            assert_eq!(p, "rawvideo,161,91,bgr24,30/1,12");
        }
        std::fs::remove_dir_all(dir).ok();
    }

    /// Con ffmpeg: ogni preset produce davvero il formato che dichiara.
    #[test]
    fn presets_produce_what_they_declare() {
        let caps = caps();
        let dir = std::env::temp_dir().join(format!("forge_video_presets_{}", std::process::id()));
        let list = frames(&dir, 3);
        for (preset, expected) in [
            (Preset::High, "h264,160,90,yuv420p,30/1,3"),
            (Preset::VeryHigh, "h264,160,90,yuv420p,30/1,3"),
            (Preset::Lossless, "ffv1,160,90,bgr0,30/1,3"),
            (Preset::Editing, "prores,160,90,yuv422p10le,30/1,3"),
        ] {
            if !VideoSettings::preset_available(preset, caps) || !caps.has(Codec::H264) {
                continue;
            }
            let mut s = VideoSettings::default();
            s.apply_preset(preset, caps);
            let out = dir.join(format!("{preset:?}.tmpnome"));
            let r = encode(&job(list.clone(), out, s, 1), &Progress::default()).unwrap();
            assert_eq!(r.path.extension().unwrap(), r.container.extension());
            assert_eq!(r.encoder, "ffmpeg");
            check_first_frame(&r.path);
            if let Some(p) = probe(&r.path) {
                assert_eq!(p, expected, "{preset:?}");
            }
        }
        std::fs::remove_dir_all(dir).ok();
    }

    /// Un'interruzione non lascia né il video né il temporaneo.
    #[test]
    fn a_cancelled_video_leaves_no_file() {
        let dir = std::env::temp_dir().join(format!("forge_video_stop_{}", std::process::id()));
        let list = frames(&dir, 3);
        let mut s = VideoSettings::default();
        s.apply_preset(Preset::Uncompressed, &Caps::default());
        let out = dir.join("v.avi");
        let progress = Progress::default();
        progress.cancel.store(true, Ordering::Relaxed);
        assert!(encode(&job(list, out.clone(), s, 1), &progress).is_err());
        assert!(!out.exists());
        assert!(!out.with_extension("avi.tmp").exists());
        std::fs::remove_dir_all(dir).ok();
    }

    /// Le scelte impossibili si correggono alla vista, non di nascosto alla codifica.
    #[test]
    fn settings_stay_consistent() {
        let none = Caps::default();
        let mut s = VideoSettings {
            container: Container::Avi,
            ..Default::default()
        };
        s.sanitize(&none);
        assert_eq!((s.codec, s.pixel), (Codec::Raw, Pixel::Bgr24));
        assert!(!none.can(Container::Mkv, Codec::Ffv1));
        assert!(!VideoSettings::preset_available(Preset::Lossless, &none));
        let mut s = VideoSettings {
            rate: Rate::Crf,
            pixel: Pixel::Yuv444,
            ..Default::default()
        };
        s.sanitize(&none);
        assert_eq!((s.rate, s.pixel), (Rate::Bitrate, Pixel::Yuv420));
        // I lati pari valgono per i codec compressi, non per il non compresso.
        assert_eq!(VideoSettings::default().output_size(161, 91), (160, 90));
        let e = VideoSettings {
            container: Container::Avi,
            codec: Codec::Raw,
            pixel: Pixel::Bgr24,
            ..Default::default()
        }
        .estimate(161, 91, 30, 10)
        .unwrap();
        assert!(e.exact);
        assert_eq!(e.bytes, 10 * 484 * 91);
    }

    #[test]
    fn bitrate_scales_with_size() {
        assert_eq!(bitrate_for(1920, 1080, 30), 12_441_600);
        assert_eq!(bitrate_for(64, 64, 30), 4_000_000);
        assert_eq!(VideoSettings::default().bitrate(1920, 1080, 30), Some(12_441_600));
    }

    #[test]
    fn the_estimate_waits_for_real_data() {
        use std::time::Duration;
        let s = Duration::from_secs(10);
        assert_eq!(eta(0, 100, s), None);
        assert_eq!(eta(1, 100, s), None);
        assert_eq!(eta(2, 100, s), Some(Duration::from_secs(490)));
        assert_eq!(eta(100, 100, s), Some(Duration::ZERO));
    }
}
