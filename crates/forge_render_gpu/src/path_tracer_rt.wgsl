// ================================================================ Ray query (RT core)
// Variante del tracciamento con le unità di ray tracing della scheda (Vulkan, ray query di wgpu):
// sostituisce `trace` (quello con la BVH dello shader diventa `trace_bvh`); tutto il resto dello
// shader è identico. Stessi triangoli, nello stesso ordine: `primitive_index` è l'indice della
// primitiva della scena e le coordinate baricentriche hanno la stessa convenzione (v0, v0+e1, v0+e2).
//
// Ritagli (texture di opacità): le ray query di wgpu non hanno ancora le intersezioni "candidate",
// quindi tutti i triangoli sono opachi per l'hardware; se il punto colpito è trasparente si riparte
// subito oltre, finché si trova un punto pieno.

@group(0) @binding(5) var ACC: acceleration_structure;

fn trace(o: vec3<f32>, d: vec3<f32>, tmin: f32, tmax_in: f32, any_hit: bool) -> Hit {
    var h: Hit;
    h.found = false;
    h.t = tmax_in;
    let cutout = pu(P_HAS_CUTOUT) != 0u;
    var flags = RAY_FLAG_NONE;
    if (any_hit && !cutout) { flags = RAY_FLAG_TERMINATE_ON_FIRST_HIT; }
    var t0 = tmin;
    for (var k: u32 = 0u; k < 32u; k = k + 1u) {
        if (!(t0 < h.t)) { return h; }
        var rq: ray_query;
        rayQueryInitialize(&rq, ACC, RayDesc(flags, 0xFFu, t0, min(h.t, 1e30), o, d));
        rayQueryProceed(&rq);
        let it = rayQueryGetCommittedIntersection(&rq);
        if (it.kind == RAY_QUERY_INTERSECTION_NONE) { return h; }
        let prim = it.primitive_index;
        if (!cutout || opaque_at(prim, it.t, it.barycentrics.x, it.barycentrics.y, o, d)) {
            h.t = it.t;
            h.u = it.barycentrics.x;
            h.v = it.barycentrics.y;
            h.prim = prim;
            h.found = true;
            return h;
        }
        // Punto trasparente: si cerca di nuovo appena oltre.
        t0 = it.t + max(1e-6, it.t * 1e-6);
    }
    return h;
}
