//! Modello dati della scena di ForgeRender.
//!
//! La scena è indipendente dal backend di rendering e dal formato di origine.
//! L'unità interna di lunghezza è il **metro**: gli importatori convertono
//! dalle unità del file ([`LengthUnit`]).

pub mod added;
pub mod camera;
pub mod environment;
pub mod fog;
pub mod light;
pub mod material;
pub mod mesh;
pub mod primitives;
pub mod scene;
pub mod units;

pub use added::{AddedKind, AddedRecord};
pub use camera::{Camera, OrbitController, RenderCamera};
pub use environment::Environment;
pub use fog::{FogPreset, FogVolume, FOG_PRESETS, FOG_QUALITY};
pub use light::{Light, LightKind};
pub use material::{Material, MaterialId, MaterialLibrary, Projection, TextureAnchor, TextureMapping, LIQUIDS_CATEGORY};
pub use mesh::Mesh;
pub use scene::{ComponentId, CopyRecord, Node, NodeIndex, Scene, UserTransform};
pub use units::LengthUnit;
