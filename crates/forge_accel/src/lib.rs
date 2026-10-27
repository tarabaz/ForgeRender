//! BVH a due figli costruita con la Surface Area Heuristic (SAH) a bin.
//!
//! Il layout dei nodi è piatto e `#[repr(C)]` (32 byte, allineato alle linee
//! di cache e direttamente caricabile in uno storage buffer GPU nella Fase 4).

mod bvh;
mod triangles;

pub use bvh::{Bvh, BvhNode, BvhStats};
pub use triangles::{Triangle, TriangleAccel, TriangleHit};
