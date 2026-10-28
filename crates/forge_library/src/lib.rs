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

pub mod detect;
pub mod online;
pub mod paths;
pub mod store;
