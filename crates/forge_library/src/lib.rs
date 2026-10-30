//! Libreria di ForgeRender: materiali PBR con texture e mappe HDRI.
//!
//! - [`paths`]: cartelle `Libreria/HDRI`, `Libreria/Materiali`, `Libreria/Texture`
//!   (accanto all'eseguibile nella versione portable, in Documenti/ForgeRender
//!   nella versione installata).
//! - [`detect`]: riconoscimento delle mappe PBR dai nomi dei file (ambientCG,
//!   Poly Haven, Substance, Quixel, …), comprese le mappe ARM/ORM impacchettate.
//! - [`store`]: materiali salvati nella libreria (cartella con le mappe +
//!   `materiale.json` + miniatura), import da cartella o da zip.
//! - [`online`]: ricerca e download da ambientCG e Poly Haven (CC0).
//! - [`mtlx`]: valori scritti nei file MaterialX che accompagnano i materiali scaricati.
//! - [`blend`] e [`blenderkit`]: lettura dei file `.blend` (senza Blender) e import dei materiali
//!   BlenderKit scaricati sul PC nella libreria locale.
//! - [`categories`]: nomi in italiano delle categorie dei siti (solo per l'interfaccia).
//! - [`polyhaven_local`]: import dei materiali Poly Haven scaricati in locale con lo script
//!   di `tools/material-library-polyhaven`.

pub mod blend;
pub mod blenderkit;
pub mod categories;
pub mod detect;
pub mod mtlx;
pub mod online;
pub mod paths;
pub mod polyhaven_local;
pub mod store;
