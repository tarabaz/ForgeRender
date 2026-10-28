//! Oggetti aggiunti dall'utente (pavimento, fondale, forme semplici). Non vengono da un
//! file sorgente: il progetto salva tipo, dimensioni e posizione e li ricrea all'apertura.

use crate::mesh::Mesh;
use crate::primitives;
use crate::scene::{ComponentId, NodeIndex, Scene};
use forge_math::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AddedKind {
    /// Pavimento quadrato di lato `size`.
    Floor { size: f32 },
    /// Fondale da studio curvo.
    Backdrop { width: f32, depth: f32, height: f32, radius: f32 },
    /// Parete verticale (normale verso +Z).
    Wall { width: f32, height: f32 },
    Box { size: Vec3 },
    Sphere { radius: f32 },
    Cylinder { radius: f32, height: f32 },
    /// Tazza / contenitore cavo.
    Cup { radius: f32, height: f32 },
}

impl AddedKind {
    pub fn label(&self) -> &'static str {
        match self {
            AddedKind::Floor { .. } => "Pavimento",
            AddedKind::Backdrop { .. } => "Fondale da studio",
            AddedKind::Wall { .. } => "Parete",
            AddedKind::Box { .. } => "Cubo",
            AddedKind::Sphere { .. } => "Sfera",
            AddedKind::Cylinder { .. } => "Cilindro",
            AddedKind::Cup { .. } => "Tazza",
        }
    }

    /// Mesh con la base a y = 0 (la sfera e il cubo appoggiano sul piano).
    pub fn mesh(&self) -> Mesh {
        match *self {
            AddedKind::Floor { size } => primitives::plane(size, size),
            AddedKind::Backdrop {
                width,
                depth,
                height,
                radius,
            } => primitives::backdrop(width, depth, height, radius, 24),
            AddedKind::Wall { width, height } => {
                let mut m = primitives::plane(width, height);
                // Piano XZ ruotato in verticale, rivolto verso +Z, base a y = 0.
                let r = Mat4::from_translation(Vec3::Y * height * 0.5) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
                for p in &mut m.positions {
                    *p = r.transform_point3(*p);
                }
                for n in &mut m.normals {
                    *n = r.transform_vector3(*n);
                }
                m.name = "Parete".into();
                m
            }
            AddedKind::Box { size } => {
                let mut m = primitives::cuboid(size);
                for p in &mut m.positions {
                    p.y += size.y * 0.5;
                }
                m
            }
            AddedKind::Sphere { radius } => {
                let mut m = primitives::uv_sphere(radius, 64, 32);
                for p in &mut m.positions {
                    p.y += radius;
                }
                m
            }
            AddedKind::Cylinder { radius, height } => primitives::cylinder(radius, height, 64),
            AddedKind::Cup { radius, height } => primitives::cup(radius, height, (radius * 0.06).max(0.002), 64),
        }
    }
}

/// Oggetto aggiunto: id del componente, nome, tipo e posizione iniziale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddedRecord {
    pub id: ComponentId,
    pub name: String,
    pub kind: AddedKind,
    pub placement: Mat4,
}

impl Scene {
    /// Crea il componente di un oggetto aggiunto (anche per ricrearlo da un progetto).
    pub fn instantiate_added(&mut self, r: &AddedRecord) -> NodeIndex {
        let mesh = self.add_mesh(r.kind.mesh());
        let i = self.add_node(None, r.id.clone(), r.name.clone(), r.placement, Some(mesh));
        self.refresh_pivots();
        i
    }

    /// Aggiunge un oggetto alla scena con un nome libero ("Pavimento", "Pavimento 2", …).
    pub fn add_object(&mut self, kind: AddedKind, placement: Mat4) -> NodeIndex {
        let base = kind.label();
        let mut n = 1;
        let (id, name) = loop {
            let id = ComponentId(format!("aggiunti/{}{}", base.to_lowercase().replace(' ', "-"), if n == 1 { String::new() } else { format!("-{n}") }));
            if self.find(&id).is_none() {
                let name = if n == 1 { base.to_string() } else { format!("{base} {n}") };
                break (id, name);
            }
            n += 1;
        };
        let r = AddedRecord {
            id,
            name,
            kind,
            placement,
        };
        let i = self.instantiate_added(&r);
        self.added.push(r);
        i
    }

    /// Vero se il componente è un oggetto aggiunto dall'utente.
    pub fn is_added(&self, idx: NodeIndex) -> bool {
        self.added.iter().any(|r| r.id == self.nodes[idx].id)
    }

    /// Elimina un oggetto aggiunto. Restituisce falso se non lo è.
    pub fn delete_added(&mut self, idx: NodeIndex) -> bool {
        let id = self.nodes[idx].id.clone();
        if !self.added.iter().any(|r| r.id == id) {
            return false;
        }
        // Prima le copie fatte da questo oggetto (all'apertura non avrebbero più l'originale).
        let deps: Vec<ComponentId> = self
            .copies
            .iter()
            .filter(|c| c.of == id || c.of.0.starts_with(&format!("{}/", id.0)))
            .map(|c| c.id.clone())
            .collect();
        for d in deps {
            if let Some(i) = self.find(&d) {
                self.delete_copy(i);
            }
        }
        let Some(idx) = self.find(&id) else { return false };
        self.remove_subtree(idx);
        self.assignments.remove(&id);
        self.added.retain(|r| r.id != id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_objects_get_unique_names_and_can_be_deleted() {
        let mut s = Scene::default();
        let a = s.add_object(AddedKind::Floor { size: 4.0 }, Mat4::IDENTITY);
        let b = s.add_object(AddedKind::Floor { size: 4.0 }, Mat4::IDENTITY);
        assert_eq!(s.nodes[a].name, "Pavimento");
        assert_eq!(s.nodes[b].name, "Pavimento 2");
        assert!(s.is_added(b));
        let sphere = s.add_object(AddedKind::Sphere { radius: 0.5 }, Mat4::IDENTITY);
        // La sfera appoggia su y = 0.
        assert!(s.exact_bounds_of(sphere).min.y.abs() < 1e-5);
        let id_b = s.nodes[b].id.clone();
        assert!(s.delete_added(s.find(&id_b).unwrap()));
        assert!(s.find(&id_b).is_none());
        assert_eq!(s.added.len(), 2);
    }
}
