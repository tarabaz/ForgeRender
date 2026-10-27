use forge_math::{Aabb, Vec3};

/// Nodo BVH. Se `count == 0` è un nodo interno con figli in `left_or_first`
/// e `left_or_first + 1`; altrimenti è una foglia con `count` primitive a
/// partire da `left_or_first`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BvhNode {
    pub min: [f32; 3],
    pub left_or_first: u32,
    pub max: [f32; 3],
    pub count: u32,
}

impl BvhNode {
    #[inline(always)]
    pub fn is_leaf(&self) -> bool {
        self.count > 0
    }

    #[inline(always)]
    pub fn aabb(&self) -> Aabb {
        Aabb::new(Vec3::from_array(self.min), Vec3::from_array(self.max))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    /// Permutazione: la primitiva in posizione `i` dell'ordine BVH è `prim_indices[i]`.
    pub prim_indices: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BvhStats {
    pub nodes: usize,
    pub leaves: usize,
    pub max_depth: usize,
    pub max_leaf_size: usize,
    pub sah_cost: f32,
}

const BINS: usize = 16;
const MAX_LEAF: usize = 8;
const TRAVERSAL_COST: f32 = 1.0;
const INTERSECT_COST: f32 = 1.0;
pub(crate) const STACK_SIZE: usize = 64;

impl Bvh {
    /// Costruisce la BVH a partire dai box e dai centroidi delle primitive.
    pub fn build(bounds: &[Aabb], centroids: &[Vec3]) -> Bvh {
        assert_eq!(bounds.len(), centroids.len());
        let n = bounds.len();
        let mut idx: Vec<u32> = (0..n as u32).collect();
        if n == 0 {
            return Bvh {
                nodes: vec![BvhNode {
                    min: [0.0; 3],
                    max: [0.0; 3],
                    left_or_first: 0,
                    count: 0,
                }],
                prim_indices: idx,
            };
        }
        let mut nodes = Vec::with_capacity(2 * n / 2 + 1);
        nodes.push(BvhNode::default());
        // (nodo, inizio, conteggio, profondità)
        let mut stack: Vec<(usize, usize, usize, usize)> = vec![(0, 0, n, 0)];

        while let Some((ni, start, count, depth)) = stack.pop() {
            let mut nb = Aabb::EMPTY;
            let mut cb = Aabb::EMPTY;
            for &p in &idx[start..start + count] {
                nb.grow(&bounds[p as usize]);
                cb.grow_point(centroids[p as usize]);
            }
            nodes[ni].min = nb.min.to_array();
            nodes[ni].max = nb.max.to_array();

            let make_leaf = |nodes: &mut Vec<BvhNode>| {
                nodes[ni].left_or_first = start as u32;
                nodes[ni].count = count as u32;
            };
            if count <= 2 || depth >= STACK_SIZE - 2 {
                make_leaf(&mut nodes);
                continue;
            }

            // Ricerca dello split SAH migliore sui tre assi.
            let mut best: Option<(usize, usize, f32)> = None; // (asse, bin di split, costo)
            let ext = cb.extent();
            let node_area = nb.half_area().max(f32::MIN_POSITIVE);
            for axis in 0..3 {
                if ext[axis] <= 0.0 {
                    continue;
                }
                let scale = BINS as f32 / ext[axis];
                let mut bin_box = [Aabb::EMPTY; BINS];
                let mut bin_cnt = [0usize; BINS];
                for &p in &idx[start..start + count] {
                    let b = (((centroids[p as usize][axis] - cb.min[axis]) * scale) as usize).min(BINS - 1);
                    bin_cnt[b] += 1;
                    bin_box[b].grow(&bounds[p as usize]);
                }
                let mut right_area = [0.0f32; BINS];
                let mut right_cnt = [0usize; BINS];
                let mut acc = Aabb::EMPTY;
                let mut c = 0;
                for b in (1..BINS).rev() {
                    acc.grow(&bin_box[b]);
                    c += bin_cnt[b];
                    right_area[b] = if c > 0 { acc.half_area() } else { 0.0 };
                    right_cnt[b] = c;
                }
                let mut acc = Aabb::EMPTY;
                let mut c = 0;
                for b in 0..BINS - 1 {
                    acc.grow(&bin_box[b]);
                    c += bin_cnt[b];
                    let rc = right_cnt[b + 1];
                    if c == 0 || rc == 0 {
                        continue;
                    }
                    let cost = TRAVERSAL_COST + INTERSECT_COST * (acc.half_area() * c as f32 + right_area[b + 1] * rc as f32) / node_area;
                    if best.is_none_or(|(_, _, bc)| cost < bc) {
                        best = Some((axis, b + 1, cost));
                    }
                }
            }

            let leaf_cost = INTERSECT_COST * count as f32;
            let mid = match best {
                Some((_, _, cost)) if cost >= leaf_cost && count <= MAX_LEAF => {
                    make_leaf(&mut nodes);
                    continue;
                }
                Some((axis, split_bin, _)) => {
                    let scale = BINS as f32 / ext[axis];
                    let slice = &mut idx[start..start + count];
                    let mut i = 0;
                    let mut j = slice.len();
                    while i < j {
                        let p = slice[i] as usize;
                        let b = (((centroids[p][axis] - cb.min[axis]) * scale) as usize).min(BINS - 1);
                        if b < split_bin {
                            i += 1;
                        } else {
                            j -= 1;
                            slice.swap(i, j);
                        }
                    }
                    i
                }
                None => {
                    // Tutti i centroidi coincidono: nessuno split informativo.
                    if count <= MAX_LEAF {
                        make_leaf(&mut nodes);
                        continue;
                    }
                    count / 2
                }
            };
            let mid = if mid == 0 || mid == count { count / 2 } else { mid };

            let left = nodes.len();
            nodes.push(BvhNode::default());
            nodes.push(BvhNode::default());
            nodes[ni].left_or_first = left as u32;
            nodes[ni].count = 0;
            stack.push((left + 1, start + mid, count - mid, depth + 1));
            stack.push((left, start, mid, depth + 1));
        }
        Bvh { nodes, prim_indices: idx }
    }

    pub fn bounds(&self) -> Aabb {
        self.nodes[0].aabb()
    }

    /// Attraversamento generico in ordine front-to-back.
    ///
    /// `leaf(first, count, t_max)` interseca le primitive di una foglia e può
    /// ridurre `t_max`; se restituisce `true` l'attraversamento termina
    /// (usato per i raggi d'ombra, "any hit").
    #[inline]
    pub fn traverse<F>(&self, origin: Vec3, dir: Vec3, t_min: f32, mut t_max: f32, mut leaf: F)
    where
        F: FnMut(u32, u32, &mut f32) -> bool,
    {
        if self.nodes[0].count == 0 && self.nodes.len() == 1 {
            return;
        }
        let inv = dir.recip();
        let mut stack = [0u32; STACK_SIZE];
        let mut sp = 0usize;
        let mut node = &self.nodes[0];
        if node.aabb().intersect(origin, inv, t_min, t_max).is_none() {
            return;
        }
        loop {
            if node.is_leaf() {
                if leaf(node.left_or_first, node.count, &mut t_max) {
                    return;
                }
            } else {
                let l = &self.nodes[node.left_or_first as usize];
                let r = &self.nodes[node.left_or_first as usize + 1];
                let tl = l.aabb().intersect(origin, inv, t_min, t_max);
                let tr = r.aabb().intersect(origin, inv, t_min, t_max);
                match (tl, tr) {
                    (Some(a), Some(b)) => {
                        let (near, far) = if a <= b {
                            (node.left_or_first, node.left_or_first + 1)
                        } else {
                            (node.left_or_first + 1, node.left_or_first)
                        };
                        stack[sp] = far;
                        sp += 1;
                        node = &self.nodes[near as usize];
                        continue;
                    }
                    (Some(_), None) => {
                        node = l;
                        continue;
                    }
                    (None, Some(_)) => {
                        node = r;
                        continue;
                    }
                    (None, None) => {}
                }
            }
            // Pop, scartando i nodi ormai più lontani dell'intersezione trovata.
            loop {
                if sp == 0 {
                    return;
                }
                sp -= 1;
                let n = &self.nodes[stack[sp] as usize];
                if n.aabb().intersect(origin, inv, t_min, t_max).is_some() {
                    node = n;
                    break;
                }
            }
        }
    }

    pub fn stats(&self) -> BvhStats {
        let mut s = BvhStats {
            nodes: self.nodes.len(),
            ..Default::default()
        };
        let root_area = self.nodes[0].aabb().half_area().max(f32::MIN_POSITIVE);
        let mut stack = vec![(0usize, 1usize)];
        while let Some((i, d)) = stack.pop() {
            let n = &self.nodes[i];
            s.max_depth = s.max_depth.max(d);
            let rel = n.aabb().half_area() / root_area;
            if n.is_leaf() {
                s.leaves += 1;
                s.max_leaf_size = s.max_leaf_size.max(n.count as usize);
                s.sah_cost += rel * INTERSECT_COST * n.count as f32;
            } else if self.nodes.len() > 1 {
                s.sah_cost += rel * TRAVERSAL_COST;
                stack.push((n.left_or_first as usize, d + 1));
                stack.push((n.left_or_first as usize + 1, d + 1));
            }
        }
        s
    }
}
