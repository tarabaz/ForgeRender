use crate::{
    camera::Camera,
    environment::Environment,
    light::Light,
    material::{MaterialId, MaterialLibrary},
    mesh::Mesh,
    primitives,
};
use forge_math::{Aabb, Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Identificatore stabile di un componente. È costruito dal percorso
/// nell'albero (es. `"staffa.step/Assieme/Vite-3"`) così da restare uguale
/// quando lo stesso file viene reimportato: è la chiave con cui il progetto
/// associa materiali e visibilità ai componenti.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ComponentId(pub String);

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub type NodeIndex = usize;

/// Nodo dell'albero di scena (parte, assieme o gruppo).
#[derive(Clone, Debug)]
pub struct Node {
    pub id: ComponentId,
    pub name: String,
    pub parent: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,
    pub local_transform: Mat4,
    pub mesh: Option<usize>,
    pub visible: bool,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub nodes: Vec<Node>,
    pub roots: Vec<NodeIndex>,
    pub meshes: Vec<Arc<Mesh>>,
    pub materials: MaterialLibrary,
    /// Assegnazioni componente → materiale. Un'assegnazione su un assieme vale
    /// per tutti i figli che non ne hanno una propria (override per istanza).
    pub assignments: BTreeMap<ComponentId, MaterialId>,
    /// Materiale dei componenti senza assegnazione.
    pub default_material: MaterialId,
    pub lights: Vec<Light>,
    pub environment: Environment,
    pub camera: Camera,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            meshes: Vec::new(),
            materials: MaterialLibrary::with_presets(),
            assignments: BTreeMap::new(),
            default_material: MaterialId::new("default"),
            lights: Vec::new(),
            environment: Environment::default(),
            camera: Camera::default(),
        }
    }
}

impl Scene {
    pub fn add_mesh(&mut self, mesh: Mesh) -> usize {
        self.meshes.push(Arc::new(mesh));
        self.meshes.len() - 1
    }

    /// Aggiunge un nodo. L'id deve essere univoco: se esiste già viene reso
    /// univoco con un suffisso (e viene registrato un avviso).
    pub fn add_node(
        &mut self,
        parent: Option<NodeIndex>,
        id: ComponentId,
        name: impl Into<String>,
        local_transform: Mat4,
        mesh: Option<usize>,
    ) -> NodeIndex {
        let mut id = id;
        if self.find(&id).is_some() {
            let base = id.0.clone();
            let mut n = 2;
            while self.find(&ComponentId(format!("{base}#{n}"))).is_some() {
                n += 1;
            }
            id = ComponentId(format!("{base}#{n}"));
            tracing::warn!("id componente duplicato, rinominato in {id}");
        }
        let idx = self.nodes.len();
        self.nodes.push(Node {
            id,
            name: name.into(),
            parent,
            children: Vec::new(),
            local_transform,
            mesh,
            visible: true,
        });
        match parent {
            Some(p) => self.nodes[p].children.push(idx),
            None => self.roots.push(idx),
        }
        idx
    }

    pub fn find(&self, id: &ComponentId) -> Option<NodeIndex> {
        self.nodes.iter().position(|n| &n.id == id)
    }

    pub fn world_transform(&self, mut idx: NodeIndex) -> Mat4 {
        let mut m = self.nodes[idx].local_transform;
        while let Some(p) = self.nodes[idx].parent {
            m = self.nodes[p].local_transform * m;
            idx = p;
        }
        m
    }

    /// Visibile se il nodo e tutti i suoi antenati sono visibili.
    pub fn is_visible(&self, mut idx: NodeIndex) -> bool {
        loop {
            if !self.nodes[idx].visible {
                return false;
            }
            match self.nodes[idx].parent {
                Some(p) => idx = p,
                None => return true,
            }
        }
    }

    /// Materiale effettivo: assegnazione propria, poi del primo antenato, poi predefinito.
    pub fn resolve_material(&self, mut idx: NodeIndex) -> &MaterialId {
        loop {
            if let Some(m) = self.assignments.get(&self.nodes[idx].id) {
                if self.materials.get(m).is_some() {
                    return m;
                }
            }
            match self.nodes[idx].parent {
                Some(p) => idx = p,
                None => return &self.default_material,
            }
        }
    }

    /// Indici dei nodi in ordine di visita (profondità), con livello di annidamento.
    pub fn walk(&self) -> Vec<(NodeIndex, usize)> {
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<(NodeIndex, usize)> = self.roots.iter().rev().map(|&r| (r, 0)).collect();
        while let Some((i, d)) = stack.pop() {
            out.push((i, d));
            for &c in self.nodes[i].children.iter().rev() {
                stack.push((c, d + 1));
            }
        }
        out
    }

    /// Bounding box in spazio mondo dei componenti visibili.
    pub fn bounds(&self) -> Aabb {
        let mut b = Aabb::EMPTY;
        for (i, node) in self.nodes.iter().enumerate() {
            if let Some(m) = node.mesh {
                if self.is_visible(i) {
                    b.grow(&self.meshes[m].bounds().transformed(&self.world_transform(i)));
                }
            }
        }
        b
    }

    pub fn bounds_of(&self, idx: NodeIndex) -> Aabb {
        let mut b = Aabb::EMPTY;
        let mut stack = vec![idx];
        while let Some(i) = stack.pop() {
            if let Some(m) = self.nodes[i].mesh {
                b.grow(&self.meshes[m].bounds().transformed(&self.world_transform(i)));
            }
            stack.extend(&self.nodes[i].children);
        }
        b
    }

    pub fn triangle_count(&self) -> usize {
        self.nodes
            .iter()
            .filter_map(|n| n.mesh)
            .map(|m| self.meshes[m].triangle_count())
            .sum()
    }

    /// Rimuove tutti i nodi sotto una radice (es. per reimportare un file),
    /// ricompattando gli indici. Le mesh non più referenziate restano in memoria
    /// fino al prossimo `compact_meshes`.
    pub fn remove_subtree(&mut self, root: NodeIndex) {
        let mut doomed = vec![false; self.nodes.len()];
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            doomed[i] = true;
            stack.extend(&self.nodes[i].children);
        }
        let mut remap = vec![usize::MAX; self.nodes.len()];
        let mut kept = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.drain(..).enumerate() {
            if !doomed[i] {
                remap[i] = kept.len();
                kept.push(n);
            }
        }
        for n in &mut kept {
            n.parent = n.parent.map(|p| remap[p]).filter(|&p| p != usize::MAX);
            n.children = n.children.iter().map(|&c| remap[c]).filter(|&c| c != usize::MAX).collect();
        }
        self.roots = self.roots.iter().map(|&r| remap[r]).filter(|&r| r != usize::MAX).collect();
        self.nodes = kept;
        self.compact_meshes();
    }

    pub fn compact_meshes(&mut self) {
        let mut used = vec![usize::MAX; self.meshes.len()];
        let mut new_meshes = Vec::new();
        for n in &mut self.nodes {
            if let Some(m) = n.mesh {
                if used[m] == usize::MAX {
                    used[m] = new_meshes.len();
                    new_meshes.push(self.meshes[m].clone());
                }
                n.mesh = Some(used[m]);
            }
        }
        self.meshes = new_meshes;
    }

    /// Scena dimostrativa costruita con primitive procedurali: pavimento,
    /// sfera metallica, sfera di vetro, box in plastica, toro in rame, luci da studio.
    pub fn test_scene() -> Scene {
        let mut s = Scene::default();
        let root = s.add_node(None, ComponentId("test/Scena".into()), "Scena di test", Mat4::IDENTITY, None);
        let items: [(&str, &str, crate::mesh::Mesh, Mat4, &str); 5] = [
            ("Pavimento", "pavimento", primitives::plane(6.0, 6.0), Mat4::IDENTITY, "default"),
            (
                "Sfera acciaio",
                "sfera-acciaio",
                primitives::uv_sphere(0.5, 64, 32),
                Mat4::from_translation(Vec3::new(-1.1, 0.5, 0.0)),
                "acciaio-lucidato",
            ),
            (
                "Sfera vetro",
                "sfera-vetro",
                primitives::uv_sphere(0.45, 64, 32),
                Mat4::from_translation(Vec3::new(0.0, 0.45, 0.8)),
                "vetro",
            ),
            (
                "Box plastica",
                "box-plastica",
                primitives::cuboid(Vec3::new(0.7, 0.7, 0.7)),
                Mat4::from_translation(Vec3::new(0.2, 0.35, -0.6)) * Mat4::from_rotation_y(0.5),
                "plastica-lucida-rossa",
            ),
            (
                "Toro rame",
                "toro-rame",
                primitives::torus(0.35, 0.12, 64, 24),
                Mat4::from_translation(Vec3::new(1.2, 0.12, 0.2)),
                "rame",
            ),
        ];
        for (name, id, mesh, xf, mat) in items {
            let mi = s.add_mesh(mesh);
            let cid = ComponentId(format!("test/Scena/{id}"));
            s.add_node(Some(root), cid.clone(), name, xf, Some(mi));
            s.assignments.insert(cid, MaterialId::new(mat));
        }
        s.lights = crate::light::preset("studio", Vec3::new(0.0, 0.5, 0.0), 1.2).unwrap_or_default();
        s.camera = Camera {
            position: Vec3::new(0.3, 1.6, 4.2),
            target: Vec3::new(0.0, 0.35, 0.0),
            up: Vec3::Y,
            vfov_deg: 38.0,
        };
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_material_inheritance_and_visibility() {
        let mut s = Scene::default();
        let a = s.add_node(None, ComponentId("a".into()), "A", Mat4::from_translation(Vec3::X), None);
        let b = s.add_node(Some(a), ComponentId("a/b".into()), "B", Mat4::from_translation(Vec3::Y), None);
        let c = s.add_node(Some(b), ComponentId("a/b/c".into()), "C", Mat4::IDENTITY, None);
        assert_eq!(s.world_transform(c).transform_point3(Vec3::ZERO), Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(s.resolve_material(c), &MaterialId::new("default"));
        s.assignments.insert(ComponentId("a".into()), MaterialId::new("rame"));
        assert_eq!(s.resolve_material(c), &MaterialId::new("rame"));
        s.assignments.insert(ComponentId("a/b/c".into()), MaterialId::new("vetro"));
        assert_eq!(s.resolve_material(c), &MaterialId::new("vetro"));
        // Materiale inesistente: si torna all'antenato.
        s.assignments.insert(ComponentId("a/b/c".into()), MaterialId::new("non-esiste"));
        assert_eq!(s.resolve_material(c), &MaterialId::new("rame"));
        s.nodes[b].visible = false;
        assert!(!s.is_visible(c));
        assert!(s.is_visible(a));
    }

    #[test]
    fn duplicate_ids_are_made_unique() {
        let mut s = Scene::default();
        s.add_node(None, ComponentId("x".into()), "X", Mat4::IDENTITY, None);
        let i = s.add_node(None, ComponentId("x".into()), "X", Mat4::IDENTITY, None);
        assert_eq!(s.nodes[i].id.0, "x#2");
    }

    #[test]
    fn remove_subtree_remaps() {
        let mut s = Scene::test_scene();
        let extra = s.add_mesh(primitives::cuboid(Vec3::ONE));
        let r2 = s.add_node(None, ComponentId("other".into()), "Altro", Mat4::IDENTITY, Some(extra));
        let tris = s.triangle_count();
        s.remove_subtree(0);
        assert_eq!(s.nodes.len(), 1);
        assert_eq!(s.roots, vec![0]);
        assert_eq!(s.triangle_count(), 12);
        assert!(tris > 12);
        let _ = r2;
    }

    #[test]
    fn test_scene_is_valid() {
        let s = Scene::test_scene();
        assert!(s.triangle_count() > 1000);
        assert!(!s.bounds().is_empty());
        for (i, n) in s.nodes.iter().enumerate() {
            if n.mesh.is_some() {
                assert!(s.materials.get(s.resolve_material(i)).is_some(), "{}", n.name);
            }
        }
    }
}
