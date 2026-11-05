//! Modello dati della scena di ForgeRender.
//!
//! La scena è indipendente dal backend di rendering e dal formato di origine.
//! L'unità interna di lunghezza è il **metro**: gli importatori convertono
//! dalle unità del file ([`LengthUnit`]).

pub mod added;
pub mod camera;
pub mod effects;
pub mod environment;
pub mod fog;
pub mod light;
pub mod material;
pub mod mesh;
pub mod primitives;
pub mod scene;
pub mod smoke;
pub mod timeline;
pub mod units;

pub use added::{AddedKind, AddedRecord};
pub use camera::{Camera, OrbitController, RenderCamera};
pub use effects::{Decal, Dust, DustGlow, DustShape};
pub use environment::Environment;
pub use fog::{FogPreset, FogVolume, Haze, HazeKind, FOG_PRESETS, FOG_QUALITY};
pub use light::{LaserSource, Light, LightKind};
pub use material::{Material, MaterialId, MaterialLibrary, Projection, TextureAnchor, TextureMapping, LIQUIDS_CATEGORY};
pub use mesh::Mesh;
pub use scene::{ComponentId, CopyRecord, Node, NodeIndex, Scene, UserTransform};
pub use smoke::{SmokeCloud, SmokeMode, SmokePreset, SmokeShape, SmokeSim, SMOKE_PRESETS, SMOKE_SIM_CELLS};
pub use timeline::{Interp, Key, ModSlot, Modifier, Prop, Target, Timeline, Track};
pub use units::LengthUnit;
