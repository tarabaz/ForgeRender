//! Lettura minima dei file `.blend` (senza Blender): blocchi, catalogo delle strutture (SDNA)
//! e campi per nome. Serve all'import dei materiali BlenderKit ([`crate::blenderkit`]).
//!
//! Formati: intestazione classica (`BLENDER-v403`, blocchi da 24 byte) e quella di Blender 5
//! (`BLENDER17-01v0500`, blocchi da 32 byte); file compressi con zstd (più frame di seguito e
//! la tabella finale in un frame da saltare). Solo puntatori a 8 byte e little endian, cioè i
//! file salvati da Blender su PC degli ultimi quindici anni.

use anyhow::{bail, ensure, Context};
use std::collections::HashMap;
use std::io::Read;

/// Blocco del file: codice ("MA", "IM", "DATA"…), struttura, indirizzo originale, dati.
#[derive(Clone, Debug)]
pub struct Block {
    pub code: [u8; 4],
    /// Indice della struttura nell'SDNA.
    pub sdna: usize,
    /// Indirizzo in memoria al salvataggio: i puntatori del file vi fanno riferimento.
    pub old: u64,
    pub start: usize,
    pub len: usize,
    pub count: usize,
}

/// Campo di una struttura: nome senza `*` né dimensioni, tipo, posizione e dimensione.
#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: String,
    pub offset: usize,
    pub size: usize,
    pub pointer: bool,
}

pub struct Blend {
    data: Vec<u8>,
    pub blocks: Vec<Block>,
    by_old: HashMap<u64, usize>,
    /// Nome di ogni struttura (indice SDNA) e i suoi campi.
    structs: Vec<(String, Vec<Field>)>,
    struct_index: HashMap<String, usize>,
}

/// Decomprime tutti i frame zstd di seguito; i frame "skippable" (tabella di ricerca) si saltano.
fn decompress(raw: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut input = raw;
    let mut out = Vec::new();
    while input.len() >= 8 {
        let magic = u32::from_le_bytes(input[..4].try_into()?);
        if (0x184D_2A50..=0x184D_2A5F).contains(&magic) {
            let n = u32::from_le_bytes(input[4..8].try_into()?) as usize;
            input = &input[(8 + n).min(input.len())..];
            continue;
        }
        let mut d = ruzstd::StreamingDecoder::new(&mut input).context("frame zstd non valido")?;
        d.read_to_end(&mut out).context("decompressione zstd")?;
    }
    Ok(out)
}

fn u32_at(d: &[u8], o: usize) -> anyhow::Result<u32> {
    Ok(u32::from_le_bytes(d.get(o..o + 4).context("file troncato")?.try_into()?))
}

fn u64_at(d: &[u8], o: usize) -> anyhow::Result<u64> {
    Ok(u64::from_le_bytes(d.get(o..o + 8).context("file troncato")?.try_into()?))
}

impl Blend {
    pub fn open(path: &std::path::Path) -> anyhow::Result<Blend> {
        let raw = std::fs::read(path).with_context(|| format!("lettura di {}", path.display()))?;
        Blend::parse(raw)
    }

    pub fn parse(raw: Vec<u8>) -> anyhow::Result<Blend> {
        let data = if raw.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
            decompress(&raw)?
        } else {
            raw
        };
        ensure!(data.starts_with(b"BLENDER"), "non è un file .blend");
        // Intestazione: classica "BLENDER-v403" (12 byte) o nuova "BLENDER17-01v0500" (17).
        let (mut pos, new_format) = if data[7].is_ascii_digit() {
            let size: usize = std::str::from_utf8(&data[7..9])?.parse()?;
            ensure!(data.get(size - 5) == Some(&b'v'), "file .blend big endian: non supportato");
            (size, true)
        } else {
            ensure!(data[7] == b'-', "file .blend con puntatori a 4 byte: non supportato");
            ensure!(data[8] == b'v', "file .blend big endian: non supportato");
            (12, false)
        };
        let mut blocks = Vec::new();
        let mut dna: Option<(usize, usize)> = None;
        loop {
            let code: [u8; 4] = data.get(pos..pos + 4).context("file troncato")?.try_into()?;
            let (sdna, old, len, count, head) = if new_format {
                (
                    u32_at(&data, pos + 4)? as usize,
                    u64_at(&data, pos + 8)?,
                    u64_at(&data, pos + 16)? as usize,
                    u64_at(&data, pos + 24)? as usize,
                    32,
                )
            } else {
                (
                    u32_at(&data, pos + 16)? as usize,
                    u64_at(&data, pos + 8)?,
                    u32_at(&data, pos + 4)? as usize,
                    u32_at(&data, pos + 20)? as usize,
                    24,
                )
            };
            let start = pos + head;
            ensure!(start + len <= data.len(), "blocco oltre la fine del file");
            if &code == b"ENDB" {
                break;
            }
            if &code == b"DNA1" {
                dna = Some((start, len));
            }
            blocks.push(Block {
                code,
                sdna,
                old,
                start,
                len,
                count,
            });
            pos = start + len;
        }
        let (dstart, dlen) = dna.context("catalogo delle strutture (DNA1) mancante")?;
        let structs = parse_sdna(&data[dstart..dstart + dlen])?;
        let struct_index = structs.iter().enumerate().map(|(i, s)| (s.0.clone(), i)).collect();
        let by_old = blocks.iter().enumerate().map(|(i, b)| (b.old, i)).collect();
        Ok(Blend {
            data,
            blocks,
            by_old,
            structs,
            struct_index,
        })
    }

    /// Blocco a cui punta un puntatore del file.
    pub fn block_at(&self, ptr: u64) -> Option<&Block> {
        if ptr == 0 {
            return None;
        }
        self.by_old.get(&ptr).map(|&i| &self.blocks[i])
    }

    /// Nome della struttura di un blocco.
    pub fn struct_name(&self, b: &Block) -> &str {
        self.structs.get(b.sdna).map(|s| s.0.as_str()).unwrap_or("")
    }

    pub fn bytes(&self, b: &Block) -> &[u8] {
        &self.data[b.start..b.start + b.len]
    }

    /// Campo di una struttura (nome senza `*` né `[n]`).
    pub fn field(&self, ty: &str, name: &str) -> Option<&Field> {
        let s = self.struct_index.get(ty)?;
        self.structs[*s].1.iter().find(|f| f.name == name)
    }

    /// Istanza di una struttura: blocco e posizione dei suoi byte.
    pub fn at(&self, b: &Block) -> Inst<'_> {
        Inst {
            blend: self,
            ty: self.struct_name(b).to_string(),
            base: b.start,
        }
    }

    /// Istanza a cui punta `ptr` (inizio del blocco).
    pub fn deref(&self, ptr: u64) -> Option<Inst<'_>> {
        self.block_at(ptr).map(|b| self.at(b))
    }
}

/// Struttura dentro il file, letta per nome di campo.
#[derive(Clone)]
pub struct Inst<'a> {
    blend: &'a Blend,
    pub ty: String,
    base: usize,
}

impl<'a> Inst<'a> {
    fn get(&self, name: &str) -> Option<(&'a Field, usize)> {
        let f = self.blend.field(&self.ty, name)?;
        Some((f, self.base + f.offset))
    }

    /// Campo che è a sua volta una struttura (es. `id` dentro `Image`).
    pub fn sub(&self, name: &str) -> Option<Inst<'a>> {
        let (f, o) = self.get(name)?;
        Some(Inst {
            blend: self.blend,
            ty: f.ty.clone(),
            base: o,
        })
    }

    pub fn ptr(&self, name: &str) -> u64 {
        self.get(name)
            .filter(|(f, _)| f.pointer)
            .and_then(|(_, o)| u64_at(&self.blend.data, o).ok())
            .unwrap_or(0)
    }

    pub fn deref(&self, name: &str) -> Option<Inst<'a>> {
        self.blend.deref(self.ptr(name))
    }

    pub fn f32(&self, name: &str) -> Option<f32> {
        let (_, o) = self.get(name)?;
        Some(f32::from_le_bytes(self.blend.data.get(o..o + 4)?.try_into().ok()?))
    }

    /// Array di float (es. `value[4]` di un colore).
    pub fn f32s(&self, name: &str) -> Vec<f32> {
        let Some((f, o)) = self.get(name) else { return Vec::new() };
        (0..f.size / 4)
            .filter_map(|i| self.blend.data.get(o + i * 4..o + i * 4 + 4))
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect()
    }

    pub fn i32(&self, name: &str) -> Option<i32> {
        let (_, o) = self.get(name)?;
        Some(i32::from_le_bytes(self.blend.data.get(o..o + 4)?.try_into().ok()?))
    }

    /// Stringa C a lunghezza fissa (es. `name[64]`).
    pub fn str(&self, name: &str) -> String {
        let Some((f, o)) = self.get(name) else { return String::new() };
        let bytes = &self.blend.data[o..(o + f.size).min(self.blend.data.len())];
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }

    /// Elementi di una `ListBase` (campo `first`, poi `next`).
    pub fn list(&self, name: &str) -> Vec<Inst<'a>> {
        let mut out = Vec::new();
        let Some(lb) = self.sub(name) else { return out };
        let mut cur = lb.deref("first");
        while let Some(i) = cur {
            if out.len() > 10_000 {
                break;
            }
            cur = i.deref("next");
            out.push(i);
        }
        out
    }

    /// Byte del blocco a cui punta il campo (es. i dati di un `PackedFile`).
    pub fn data_of(&self, name: &str, len: usize) -> Option<&'a [u8]> {
        let b = self.blend.block_at(self.ptr(name))?;
        self.blend.data.get(b.start..b.start + len.min(b.len))
    }

    /// Indirizzo originale di questa struttura (per confrontarla con i puntatori).
    pub fn old_address(&self) -> Option<u64> {
        self.blend.blocks.iter().find(|b| b.start == self.base).map(|b| b.old)
    }
}

/// Catalogo SDNA: nomi, tipi con le dimensioni, strutture con i campi.
fn parse_sdna(d: &[u8]) -> anyhow::Result<Vec<(String, Vec<Field>)>> {
    let mut p = 0usize;
    let expect = |p: &mut usize, tag: &[u8]| -> anyhow::Result<()> {
        *p = (*p + 3) & !3;
        ensure!(d.get(*p..*p + 4) == Some(tag), "SDNA: manca {}", String::from_utf8_lossy(tag));
        *p += 4;
        Ok(())
    };
    let strings = |p: &mut usize, n: usize| -> anyhow::Result<Vec<String>> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let end = d[*p..].iter().position(|&c| c == 0).context("SDNA troncato")?;
            v.push(String::from_utf8_lossy(&d[*p..*p + end]).into_owned());
            *p += end + 1;
        }
        Ok(v)
    };
    expect(&mut p, b"SDNA")?;
    expect(&mut p, b"NAME")?;
    let n = u32_at(d, p)? as usize;
    p += 4;
    let names = strings(&mut p, n)?;
    expect(&mut p, b"TYPE")?;
    let n = u32_at(d, p)? as usize;
    p += 4;
    let types = strings(&mut p, n)?;
    expect(&mut p, b"TLEN")?;
    let tlen: Vec<usize> = (0..types.len())
        .map(|i| u16::from_le_bytes([d[p + i * 2], d[p + i * 2 + 1]]) as usize)
        .collect();
    p += types.len() * 2;
    expect(&mut p, b"STRC")?;
    let n = u32_at(d, p)? as usize;
    p += 4;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let rd = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]) as usize;
        let (ty, nf) = (rd(p), rd(p + 2));
        p += 4;
        let mut fields = Vec::with_capacity(nf);
        let mut offset = 0;
        for _ in 0..nf {
            let (fty, fname) = (rd(p), rd(p + 2));
            p += 4;
            let raw = names.get(fname).context("SDNA: nome fuori intervallo")?;
            let pointer = raw.starts_with('*') || raw.starts_with("(*");
            let count: usize = raw
                .split('[')
                .skip(1)
                .filter_map(|s| s.trim_end_matches(']').parse::<usize>().ok())
                .product::<usize>()
                .max(1);
            let unit = if pointer { 8 } else { tlen[fty] };
            let clean = raw.trim_start_matches("(*").trim_start_matches('*');
            let clean = clean.split(['[', ')']).next().unwrap_or("").to_string();
            fields.push(Field {
                name: clean,
                ty: types[fty].clone(),
                offset,
                size: unit * count,
                pointer,
            });
            offset += unit * count;
        }
        if types.get(ty).is_none() {
            bail!("SDNA: tipo fuori intervallo");
        }
        out.push((types[ty].clone(), fields));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_files_that_are_not_blend() {
        assert!(Blend::parse(b"non sono un blend".to_vec()).is_err());
        // Intestazione giusta ma file troncato prima del primo blocco.
        assert!(Blend::parse(b"BLENDER-v403".to_vec()).is_err());
        // Puntatori a 4 byte e big endian non sono supportati.
        assert!(Blend::parse(b"BLENDER_v279REND".to_vec()).is_err());
        assert!(Blend::parse(b"BLENDER-V279REND".to_vec()).is_err());
    }

    /// File minimo costruito a mano: catalogo con una struttura e un blocco che la usa.
    #[test]
    fn reads_fields_by_name() {
        let mut sdna = Vec::new();
        sdna.extend_from_slice(b"SDNANAME");
        sdna.extend_from_slice(&3u32.to_le_bytes());
        sdna.extend_from_slice(b"valore\0nome[8]\0*prossimo\0");
        while sdna.len() % 4 != 0 {
            sdna.push(0);
        }
        sdna.extend_from_slice(b"TYPE");
        sdna.extend_from_slice(&3u32.to_le_bytes());
        sdna.extend_from_slice(b"float\0char\0Prova\0");
        while sdna.len() % 4 != 0 {
            sdna.push(0);
        }
        sdna.extend_from_slice(b"TLEN");
        for l in [4u16, 1, 20] {
            sdna.extend_from_slice(&l.to_le_bytes());
        }
        while sdna.len() % 4 != 0 {
            sdna.push(0);
        }
        sdna.extend_from_slice(b"STRC");
        sdna.extend_from_slice(&1u32.to_le_bytes());
        for v in [2u16, 3, 0, 0, 1, 1, 2, 2] {
            sdna.extend_from_slice(&v.to_le_bytes());
        }
        let block = |code: &[u8; 4], sdna_i: u32, old: u64, data: &[u8]| {
            let mut b = code.to_vec();
            b.extend_from_slice(&(data.len() as u32).to_le_bytes());
            b.extend_from_slice(&old.to_le_bytes());
            b.extend_from_slice(&sdna_i.to_le_bytes());
            b.extend_from_slice(&1u32.to_le_bytes());
            b.extend_from_slice(data);
            b
        };
        let mut prova = Vec::new();
        prova.extend_from_slice(&2.5f32.to_le_bytes());
        prova.extend_from_slice(b"acciaio\0");
        prova.extend_from_slice(&0x1000u64.to_le_bytes());
        let mut file = b"BLENDER-v403".to_vec();
        file.extend(block(b"MA\0\0", 0, 0x2000, &prova));
        let mut seconda = Vec::new();
        seconda.extend_from_slice(&7.0f32.to_le_bytes());
        seconda.extend_from_slice(b"rame\0\0\0\0");
        seconda.extend_from_slice(&0u64.to_le_bytes());
        file.extend(block(b"DATA", 0, 0x1000, &seconda));
        file.extend(block(b"DNA1", 0, 0, &sdna));
        file.extend(block(b"ENDB", 0, 0, &[]));
        let b = Blend::parse(file).unwrap();
        let first = b.at(&b.blocks[0]);
        assert_eq!(first.ty, "Prova");
        assert_eq!(first.f32("valore"), Some(2.5));
        assert_eq!(first.str("nome"), "acciaio");
        let next = first.deref("prossimo").expect("puntatore risolto");
        assert_eq!((next.f32("valore"), next.str("nome")), (Some(7.0), "rame".to_string()));
        assert!(next.deref("prossimo").is_none());
    }
}
