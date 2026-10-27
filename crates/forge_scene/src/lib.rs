//! Modello dati della scena di ForgeRender.
//!
//! La scena è indipendente dal backend di rendering e dal formato di origine.
//! L'unità interna di lunghezza è il **metro**: gli importatori convertono
//! dalle unità del file ([`LengthUnit`]).

pub mod camera;
pub mod environment;
pub mod light;
pub mod material;
pub mod mesh;
pub mod primitives;
pub mod scene;
pub mod units;

pub use camera::{Camera, OrbitController};
pub use environment::Environment;
pub use light::{Light, LightKind};
pub use material::{Material, MaterialId, MaterialLibrary, Projection, TextureMapping};
pub use mesh::Mesh;
pub use scene::{ComponentId, Node, NodeIndex, Scene, UserTransform};
pub use units::LengthUnit;
