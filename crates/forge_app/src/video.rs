//! Video MP4 (H.264) dai fotogrammi PNG di una sequenza. Su Windows con Media Foundation (incluso
//! nel sistema, anche con l'encoder hardware della scheda video se c'è); se manca (edizioni "N"
//! senza Media Feature Pack) o fuori da Windows si prova `ffmpeg` dal PATH.

use anyhow::{bail, Context};
use std::path::{Path, PathBuf};

/// Bitrate scelto per una buona qualità senza file enormi: ~0,2 bit per pixel per fotogramma
/// (1920 × 1080 a 30 fps ≈ 12 Mbit/s), tra 4 e 80 Mbit/s.
pub fn bitrate_for(width: u32, height: u32, fps: u32) -> u32 {
    let bits = width as f64 * height as f64 * fps.max(1) as f64 * 0.2;
    bits.clamp(4.0e6, 80.0e6) as u32
}

/// Scrive `out` con i fotogrammi in ordine, ripetuti `loops` volte (giri del turntable), a `fps`
/// fotogrammi al secondo. Lati dispari: si toglie l'ultima riga/colonna (H.264 vuole lati pari).
/// Restituisce il nome dell'encoder usato.
pub fn encode_mp4(frames: &[PathBuf], out: &Path, fps: u32, loops: u32) -> anyhow::Result<&'static str> {
    if frames.is_empty() {
        bail!("nessun fotogramma");
    }
    let first = image::open(&frames[0]).with_context(|| format!("{}", frames[0].display()))?;
    let (w, h) = (first.width() & !1, first.height() & !1);
    if w < 16 || h < 16 {
        bail!("immagine troppo piccola per il video ({w} × {h})");
    }
    let tmp = out.with_extension("mp4.tmp");
    let _ = std::fs::remove_file(&tmp);
    #[cfg(windows)]
    {
        match media_foundation::encode(frames, &tmp, w, h, fps, loops.max(1)) {
            Ok(()) => {
                std::fs::rename(&tmp, out).with_context(|| format!("{}", out.display()))?;
                return Ok("Media Foundation");
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                tracing::warn!("Media Foundation non disponibile per il video: {e:#}; provo ffmpeg");
                if ffmpeg::encode(frames, &tmp, w, h, fps, loops.max(1)).is_err() {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(e.context("encoder H.264 di Windows non disponibile (e ffmpeg non trovato)"));
                }
            }
        }
    }
    #[cfg(not(windows))]
    ffmpeg::encode(frames, &tmp, w, h, fps, loops.max(1))?;
    std::fs::rename(&tmp, out).with_context(|| format!("{}", out.display()))?;
    Ok("ffmpeg")
}

/// Fotogramma come BGRA dall'alto, ritagliato a `w × h`.
fn load_bgra(path: &Path, w: u32, h: u32) -> anyhow::Result<Vec<u8>> {
    let img = image::open(path).with_context(|| format!("{}", path.display()))?.to_rgba8();
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

    pub fn encode(frames: &[PathBuf], out: &Path, w: u32, h: u32, fps: u32, loops: u32) -> anyhow::Result<()> {
        unsafe {
            // Il thread del video è nostro: COM multithread (S_FALSE se era già inizializzato).
            let com = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            let r = (|| -> anyhow::Result<()> {
                MFStartup(MF_VERSION, MFSTARTUP_FULL).context("MFStartup")?;
                let r = write(frames, out, w, h, fps, loops);
                let _ = MFShutdown();
                r
            })();
            if com {
                CoUninitialize();
            }
            r
        }
    }

    unsafe fn write(frames: &[PathBuf], out: &Path, w: u32, h: u32, fps: u32, loops: u32) -> anyhow::Result<()> {
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
            out_type.SetUINT32(&MF_MT_AVG_BITRATE, bitrate_for(w, h, fps))?;
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
                .context("conversione RGB → H.264")?;
            writer.BeginWriting()?;

            let len = w * h * 4;
            let mut k = 0u64;
            for _ in 0..loops {
                for path in frames {
                    let bgra = load_bgra(path, w, h)?;
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

    pub fn encode(frames: &[PathBuf], out: &Path, w: u32, h: u32, fps: u32, loops: u32) -> anyhow::Result<()> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "bgra", "-s"])
            .arg(format!("{w}x{h}"))
            .arg("-r")
            .arg(fps.to_string())
            .args([
                "-i",
                "-",
                "-c:v",
                "libx264",
                "-preset",
                "slow",
                "-crf",
                "16",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
            ])
            .arg(out)
            .stdin(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Nessuna finestra di console.
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().context("ffmpeg non trovato")?;
        {
            let stdin = child.stdin.as_mut().context("ffmpeg")?;
            for _ in 0..loops {
                for path in frames {
                    stdin.write_all(&load_bgra(path, w, h)?)?;
                }
            }
        }
        drop(child.stdin.take());
        let st = child.wait()?;
        if !st.success() {
            bail!("ffmpeg è terminato con errore ({st})");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Video da tre fotogrammi colorati: file MP4 valido (intestazione "ftyp") e, se ffprobe è
    /// installato, con il numero di fotogrammi e la durata attesi.
    #[test]
    fn frames_become_an_mp4() {
        let dir = std::env::temp_dir().join(format!("forge_video_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut frames = Vec::new();
        for (i, c) in [[255u8, 0, 0], [0, 255, 0], [0, 0, 255]].iter().enumerate() {
            // Metà alta colorata, metà bassa bianca: il video non deve uscire capovolto.
            let img = image::RgbImage::from_fn(161, 91, |_, y| image::Rgb(if y < 45 { *c } else { [255, 255, 255] }));
            let p = dir.join(format!("f{i}.png"));
            img.save(&p).unwrap();
            frames.push(p);
        }
        let out = dir.join("giro.mp4");
        let enc = encode_mp4(&frames, &out, 30, 2).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.len() > 500 && &bytes[4..8] == b"ftyp", "{enc}: {} byte", bytes.len());
        assert!(!out.with_extension("mp4.tmp").exists());
        if cfg!(windows) {
            assert_eq!(enc, "Media Foundation");
        }
        // Primo fotogramma decodificato: in alto rosso, in basso bianco.
        let png = dir.join("primo.png");
        if std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&out)
            .args(["-frames:v", "1"])
            .arg(&png)
            .status()
            .is_ok_and(|s| s.success())
        {
            let f = image::open(&png).unwrap().to_rgb8();
            let (top, bottom) = (f.get_pixel(80, 10).0, f.get_pixel(80, 80).0);
            assert!(top[0] > 200 && top[1] < 60 && top[2] < 60, "alto {top:?}");
            assert!(bottom.iter().all(|&v| v > 200), "basso {bottom:?}");
        }
        if let Ok(o) = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-count_frames",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=nb_read_frames,width,height",
                "-of",
                "csv=p=0",
            ])
            .arg(&out)
            .output()
        {
            let s = String::from_utf8_lossy(&o.stdout);
            assert_eq!(s.trim(), "160,90,6", "{enc}");
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn bitrate_scales_with_size() {
        assert_eq!(bitrate_for(1920, 1080, 30), 12_441_600);
        assert_eq!(bitrate_for(64, 64, 30), 4_000_000);
    }
}
