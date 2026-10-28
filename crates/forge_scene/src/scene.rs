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

/// Trasformazione impostata dall'utente su un componente, non distruttiva:
/// si applica sopra la posizione del file sorgente e viene salvata nel progetto.
///
/// È espressa in spazio mondo: rotazione (angoli XYZ in gradi) e scala avvengono
/// attorno al centro del componente nella posizione originale (`Node::pivot`),
/// poi si applica lo spostamento. La trasformazione di un assieme si applica
/// anche a tutti i suoi figli.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserTransform {
    /// Spostamento in metri.
    pub translation: Vec3,
    pub rotation_deg: Vec3,
    pub scale: Vec3,
}

impl Default for UserTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_deg: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl UserTransform {
    pub fn is_identity(&self) -> bool {
        *self == Self::default()
    }

    /// Matrice in spazio mondo dato il punto di rotazione/scala.
    pub fn matrix(&self, pivot: Vec3) -> Mat4 {
        if self.is_identity() {
            return Mat4::IDENTITY;
        }
        let r = self.rotation_deg;
        let rot = forge_math::Quat::from_euler(
            forge_math::glam::EulerRot::XYZ,
            r.x.to_radians(),
            r.y.to_radians(),
            r.z.to_radians(),
        );
        let scale = Vec3::new(
            if self.scale.x.abs() > 1e-9 { self.scale.x } else { 1e-9 },
            if self.scale.y.abs() > 1e-9 { self.scale.y } else { 1e-9 },
            if self.scale.z.abs() > 1e-9 { self.scale.z } else { 1e-9 },
        );
        Mat4::from_translation(self.translation + pivot) * Mat4::from_quat(rot) * Mat4::from_scale(scale) * Mat4::from_translation(-pivot)
    }
}

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
    pub user: UserTransform,
    /// Centro del componente nella posizione del file sorgente (punto di rotazione/scala).
    pub pivot: Vec3,
}

/// Copia di un componente (con il suo sotto-albero) creata dall'utente. Le copie
/// non esistono nei file sorgente: il progetto le salva così e le ricrea all'apertura.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CopyRecord {
    /// Componente copiato.
    pub of: ComponentId,
    /// Id della copia (i figli hanno id `<id>/<nome>`).
    pub id: ComponentId,
    pub name: String,
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
    /// Camera della viewport (vista libera dell'utente).
    pub camera: Camera,
    /// Camere di rendering salvate nel progetto.
    pub render_cameras: Vec<crate::camera::RenderCamera>,
    /// Camera di rendering attiva; `None` = il render usa la vista della viewport.
    pub active_camera: Option<usize>,
    /// Copie create dall'utente, nell'ordine di creazione.
    pub copies: Vec<CopyRecord>,
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
            render_cameras: Vec::new(),
            active_camera: None,
            copies: Vec::new(),
        }
    }
}

impl Scene {
    /// Camera usata dal renderer: quella di rendering attiva, altrimenti la vista.
    pub fn render_camera(&self) -> &Camera {
        self.active_camera
            .and_then(|i| self.render_cameras.get(i))
            .map(|c| &c.camera)
            .unwrap_or(&self.camera)
    }

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
            user: UserTransform::default(),
            pivot: Vec3::ZERO,
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

    /// Trasformazione mondo come nel file sorgente (senza modifiche dell'utente).
    pub fn source_world_transform(&self, mut idx: NodeIndex) -> Mat4 {
        let mut m = self.nodes[idx].local_transform;
        while let Some(p) = self.nodes[idx].parent {
            m = self.nodes[p].local_transform * m;
            idx = p;
        }
        m
    }

    /// Trasformazione mondo effettiva: le trasformazioni utente degli antenati
    /// (applicate per ultime) e del nodo stesso, sopra la posizione sorgente.
    pub fn world_transform(&self, idx: NodeIndex) -> Mat4 {
        let mut user = Mat4::IDENTITY;
        let mut k = Some(idx);
        while let Some(i) = k {
            let n = &self.nodes[i];
            if !n.user.is_identity() {
                user = n.user.matrix(n.pivot) * user;
            }
            k = n.parent;
        }
        user * self.source_world_transform(idx)
    }

    /// Ricalcola i punti di rotazione/scala (centro dei componenti nella posizione
    /// sorgente). Va chiamata dopo aver aggiunto geometria.
    pub fn refresh_pivots(&mut self) {
        let mesh_bounds: Vec<Aabb> = self.meshes.iter().map(|m| m.bounds()).collect();
        let mut bounds = vec![Aabb::EMPTY; self.nodes.len()];
        // Visita in ordine inverso di profondità: i figli prima dei genitori.
        let order = self.walk();
        for &(i, _) in order.iter().rev() {
            let mut b = Aabb::EMPTY;
            if let Some(m) = self.nodes[i].mesh {
                b.grow(&mesh_bounds[m].transformed(&self.source_world_transform(i)));
            }
            for &c in &self.nodes[i].children {
                b.grow(&bounds[c]);
            }
            bounds[i] = b;
        }
        for (i, n) in self.nodes.iter_mut().enumerate() {
            n.pivot = if bounds[i].is_empty() { Vec3::ZERO } else { bounds[i].center() };
        }
    }

    /// Sposta il componente in modo che il suo punto più basso stia a quota `y`.
    pub fn drop_to_floor(&mut self, idx: NodeIndex, y: f32) {
        let b = self.exact_bounds_of(idx);
        if !b.is_empty() {
            self.translate_world(idx, Vec3::new(0.0, y - b.min.y, 0.0));
        }
    }

    /// Porta il centro del componente sull'asse verticale dell'origine (X = Z = 0).
    pub fn center_on_origin(&mut self, idx: NodeIndex) {
        let b = self.exact_bounds_of(idx);
        if !b.is_empty() {
            let c = b.center();
            self.translate_world(idx, Vec3::new(-c.x, 0.0, -c.z));
        }
    }

    /// Duplica un componente con tutto il sotto-albero (le mesh sono condivise, non
    /// copiate) e registra la copia. Restituisce l'indice della nuova radice.
    pub fn duplicate(&mut self, idx: NodeIndex) -> NodeIndex {
        let base = self.nodes[idx].id.0.clone();
        let mut n = 1;
        let id = loop {
            let candidate = ComponentId(if n == 1 {
                format!("{base}#copia")
            } else {
                format!("{base}#copia{n}")
            });
            if self.find(&candidate).is_none() {
                break candidate;
            }
            n += 1;
        };
        let name = if n == 1 {
            format!("{} (duplicato)", self.nodes[idx].name)
        } else {
            format!("{} (duplicato {n})", self.nodes[idx].name)
        };
        let new = self.duplicate_as(idx, id.clone(), name.clone());
        self.copies.push(CopyRecord {
            of: self.nodes[idx].id.clone(),
            id,
            name,
        });
        new
    }

    /// Crea la copia con id e nome dati (usata anche per ricreare le copie di un progetto).
    pub fn duplicate_as(&mut self, idx: NodeIndex, id: ComponentId, name: String) -> NodeIndex {
        let parent = self.nodes[idx].parent;
        let new = self.copy_node(idx, parent, id, name);
        match parent {
            Some(p) => self.nodes[p].children.push(new),
            None => self.roots.push(new),
        }
        new
    }

    fn copy_node(&mut self, idx: NodeIndex, parent: Option<NodeIndex>, id: ComponentId, name: String) -> NodeIndex {
        let src = self.nodes[idx].clone();
        if let Some(m) = self.assignments.get(&src.id).cloned() {
            self.assignments.insert(id.clone(), m);
        }
        let new = self.nodes.len();
        self.nodes.push(Node {
            id: id.clone(),
            name,
            parent,
            children: Vec::new(),
            ..src.clone()
        });
        for c in src.children {
            let child_id = match self.nodes[c].id.0.strip_prefix(&src.id.0) {
                Some(rest) => ComponentId(format!("{}{rest}", id.0)),
                None => ComponentId(format!("{}/{}", id.0, self.nodes[c].name)),
            };
            let child_name = self.nodes[c].name.clone();
            let nc = self.copy_node(c, Some(new), child_id, child_name);
            self.nodes[new].children.push(nc);
        }
        new
    }

    /// Vero se il componente è una copia creata dall'utente (o sta dentro una copia).
    pub fn is_copy(&self, mut idx: NodeIndex) -> bool {
        loop {
            if self.copies.iter().any(|c| c.id == self.nodes[idx].id) {
                return true;
            }
            match self.nodes[idx].parent {
                Some(p) => idx = p,
                None => return false,
            }
        }
    }

    /// Elimina una copia (e le copie fatte da lei). Restituisce falso se non è una copia.
    pub fn delete_copy(&mut self, idx: NodeIndex) -> bool {
        let id = self.nodes[idx].id.clone();
        if !self.copies.iter().any(|c| c.id == id) {
            return false;
        }
        // Copie che dipendono da questa (copiate dalla copia o da un suo figlio).
        let mut doomed = vec![id.clone()];
        let mut changed = true;
        while changed {
            changed = false;
            for c in &self.copies {
                let depends = doomed.iter().any(|d| c.of == *d || c.of.0.starts_with(&format!("{}/", d.0)));
                if depends && !doomed.contains(&c.id) {
                    doomed.push(c.id.clone());
                    changed = true;
                }
            }
        }
        for d in &doomed {
            if let Some(i) = self.find(d) {
                self.remove_subtree(i);
            }
        }
        self.assignments
            .retain(|k, _| !doomed.iter().any(|d| *k == *d || k.0.starts_with(&format!("{}/", d.0))));
        self.copies.retain(|c| !doomed.contains(&c.id));
        true
    }

    /// Ingombro esatto: i vertici trasformati uno per uno. [`bounds_of`](Self::bounds_of)
    /// trasforma invece la scatola della mesh, che per un pezzo ruotato è più larga del
    /// pezzo (gli angoli della scatola sporgono): va bene per inquadrare, non per appoggiare.
    pub fn exact_bounds_of(&self, idx: NodeIndex) -> Aabb {
        let mut b = Aabb::EMPTY;
        let mut stack = vec![idx];
        while let Some(i) = stack.pop() {
            if let Some(m) = self.nodes[i].mesh {
                let w = self.world_transform(i);
                for p in &self.meshes[m].positions {
                    b.grow_point(w.transform_point3(*p));
                }
            }
            stack.extend(&self.nodes[i].children);
        }
        b
    }

    /// Sposta il componente di `delta` in spazio mondo, anche dentro assiemi a loro
    /// volta ruotati o scalati (lo spostamento passa per l'inversa degli antenati).
    pub fn translate_world(&mut self, idx: NodeIndex, delta: Vec3) {
        let mut ancestors = Mat4::IDENTITY;
        let mut k = self.nodes[idx].parent;
        while let Some(i) = k {
            let n = &self.nodes[i];
            if !n.user.is_identity() {
                ancestors = n.user.matrix(n.pivot) * ancestors;
            }
            k = n.parent;
        }
        let local = ancestors.inverse().transform_vector3(delta);
        self.nodes[idx].user.translation += local;
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
        s.refresh_pivots();
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
    fn user_transforms_compose_and_pivot() {
        let mut s = Scene::default();
        let m = s.add_mesh(primitives::cuboid(Vec3::ONE));
        let a = s.add_node(
            None,
            ComponentId("a".into()),
            "A",
            Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)),
            None,
        );
        let b = s.add_node(Some(a), ComponentId("a/b".into()), "B", Mat4::IDENTITY, Some(m));
        s.refresh_pivots();
        assert!((s.nodes[b].pivot - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5);
        // Rotazione attorno al proprio centro: il centro non si muove.
        s.nodes[b].user.rotation_deg = Vec3::new(0.0, 45.0, 0.0);
        assert!((s.bounds_of(b).center() - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5);
        // Scala 2: il cubo raddoppia attorno al centro.
        s.nodes[b].user = UserTransform {
            scale: Vec3::splat(2.0),
            ..Default::default()
        };
        assert!((s.bounds_of(b).extent() - Vec3::splat(2.0)).length() < 1e-5);
        // Spostamento dell'assieme: si sposta anche il figlio.
        s.nodes[a].user.translation = Vec3::new(0.0, 1.0, 0.0);
        assert!((s.bounds_of(b).center() - Vec3::new(5.0, 1.0, 0.0)).length() < 1e-5);
        // Appoggia sul pavimento e centra.
        s.drop_to_floor(a, 0.0);
        assert!(s.bounds_of(b).min.y.abs() < 1e-5);
        s.center_on_origin(a);
        let c = s.bounds_of(b).center();
        assert!(c.x.abs() < 1e-5 && c.z.abs() < 1e-5, "{c}");
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
    fn duplicate_and_delete_copies() {
        let mut s = Scene::default();
        let m = s.add_mesh(crate::primitives::cuboid(Vec3::ONE));
        let asm = s.add_node(None, ComponentId("f/A".into()), "A", Mat4::IDENTITY, None);
        let p = s.add_node(Some(asm), ComponentId("f/A/P".into()), "P", Mat4::IDENTITY, Some(m));
        s.assignments.insert(ComponentId("f/A/P".into()), crate::MaterialId::new("rosso"));
        s.refresh_pivots();
        s.nodes[asm].user.translation = Vec3::new(1.0, 0.0, 0.0);
        let c = s.duplicate(asm);
        assert_eq!(s.nodes[c].id.0, "f/A#copia");
        assert_eq!(s.nodes[c].name, "A (duplicato)");
        assert_eq!(s.nodes[c].user.translation, Vec3::new(1.0, 0.0, 0.0));
        let cp = s.nodes[c].children[0];
        assert_eq!(s.nodes[cp].id.0, "f/A#copia/P");
        assert_eq!(s.nodes[cp].mesh, s.nodes[p].mesh, "la mesh è condivisa");
        assert_eq!(
            s.assignments.get(&ComponentId("f/A#copia/P".into())),
            Some(&crate::MaterialId::new("rosso"))
        );
        assert!(s.is_copy(cp) && !s.is_copy(p));
        // Copia della copia, poi eliminazione della prima: sparisce anche la seconda.
        let c2 = s.duplicate(c);
        assert_eq!(s.nodes[c2].id.0, "f/A#copia#copia");
        assert_eq!(s.copies.len(), 2);
        assert!(!s.delete_copy(p), "un pezzo del file non si elimina");
        let c = s.find(&ComponentId("f/A#copia".into())).unwrap();
        assert!(s.delete_copy(c));
        assert!(s.copies.is_empty());
        assert_eq!(s.nodes.len(), 2);
        assert!(s.assignments.keys().all(|k| !k.0.contains("#copia")));
        assert_eq!(s.roots.len(), 1);
    }

    #[test]
    fn drop_to_floor_touches_with_real_vertices() {
        // Toroide ruotato di 40° dentro un assieme a sua volta ruotato: la scatola ruotata
        // sporge sotto il pezzo, ma l'appoggio deve usare i vertici veri.
        let mut s = Scene::default();
        let m = s.add_mesh(crate::primitives::torus(0.5, 0.15, 64, 32));
        let asm = s.add_node(None, ComponentId("a".into()), "A", Mat4::IDENTITY, None);
        let t = s.add_node(
            Some(asm),
            ComponentId("a/t".into()),
            "T",
            Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            Some(m),
        );
        s.refresh_pivots();
        s.nodes[asm].user.rotation_deg = Vec3::new(0.0, 30.0, 0.0);
        s.nodes[t].user.rotation_deg = Vec3::new(0.0, 0.0, 40.0);
        let loose = s.bounds_of(t).min.y;
        let exact = s.exact_bounds_of(t).min.y;
        assert!(loose < exact - 0.05, "la scatola ruotata sporge: {loose} vs {exact}");
        s.drop_to_floor(t, 0.0);
        assert!(s.exact_bounds_of(t).min.y.abs() < 1e-5, "{}", s.exact_bounds_of(t).min.y);
        let c0 = s.exact_bounds_of(t).center();
        s.center_on_origin(t);
        let c = s.exact_bounds_of(t).center();
        assert!(c.x.abs() < 1e-5 && c.z.abs() < 1e-5 && (c.y - c0.y).abs() < 1e-5, "{c}");
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
