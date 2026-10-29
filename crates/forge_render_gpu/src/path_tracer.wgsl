// ForgeRender — path tracer GPU (compute shader).
//
// Rispecchia l'integratore CPU (forge_render_cpu): stesse BSDF (Lambert + GGX
// con compensazione multi-scattering, vetro liscio e satinato), NEE con MIS su
// luci analitiche, HDRI ed emissivi, Russian roulette, texture.
// Tutta la scena è in un unico buffer di parole a 32 bit (vedi layout.rs).

@group(0) @binding(0) var<uniform> P: array<vec4<u32>, 20>;
@group(0) @binding(1) var<storage, read> S: array<u32>;
@group(0) @binding(2) var<storage, read_write> OUT: array<vec4<f32>>;
// Campionamento adattivo: un bit per riquadro di 16×16 pixel (1 = riceve ancora campioni).
@group(0) @binding(3) var<storage, read> MASK: array<u32>;

const PI: f32 = 3.14159265358979;
const INV_PI: f32 = 0.318309886183791;
const TAU: f32 = 6.28318530717959;
const INF: f32 = 3.4e38;

// Indici dei parametri (devono coincidere con layout.rs).
const P_WIDTH: u32 = 0u;
const P_HEIGHT: u32 = 1u;
const P_FIRST_SAMPLE: u32 = 2u;
const P_N_SAMPLES: u32 = 3u;
const P_SEED_LO: u32 = 4u;
const P_SEED_HI: u32 = 5u;
const P_MAX_BOUNCES: u32 = 6u;
const P_RR_START: u32 = 7u;
const P_FLAGS: u32 = 8u;
const P_HAS_CUTOUT: u32 = 67u;
const P_LENS_R: u32 = 68u;
const P_FOCUS: u32 = 69u;
const P_BLADES: u32 = 70u;
const P_BLADE_ROT: u32 = 71u;
const P_SHIFT_X: u32 = 72u;
const P_SHIFT_Y: u32 = 73u;
const P_OFF_FOGS: u32 = 74u;
const P_N_FOGS: u32 = 75u;
const FOG_WORDS: u32 = 16u;

// Punto dell'apertura (come forge_render_cpu::sample_aperture): disco o poligono regolare.
fn sample_aperture(u0: f32, u1: f32, u2: f32) -> vec2<f32> {
    let blades = pu(P_BLADES);
    if (blades < 3u) {
        // Mappatura concentrica di Shirley-Chiu.
        let a = 2.0 * u1 - 1.0;
        let b = 2.0 * u2 - 1.0;
        if (a == 0.0 && b == 0.0) { return vec2<f32>(0.0); }
        var rr: f32;
        var th: f32;
        if (abs(a) > abs(b)) { rr = a; th = 0.78539816 * (b / a); }
        else { rr = b; th = 1.57079633 - 0.78539816 * (a / b); }
        return rr * vec2<f32>(cos(th), sin(th));
    }
    let n = f32(blades);
    let k = min(floor(u0 * n), n - 1.0);
    let step = 6.28318531 / n;
    let a0 = pf(P_BLADE_ROT) + k * step;
    let a1 = a0 + step;
    let su = sqrt(u1);
    return vec2<f32>(cos(a0), sin(a0)) * (su * (1.0 - u2)) + vec2<f32>(cos(a1), sin(a1)) * (su * u2);
}
const P_N_LIGHTS: u32 = 9u;
const P_ENV_KIND: u32 = 10u;
const P_N_EMISSIVE: u32 = 11u;
const P_CLAMP: u32 = 12u;
const P_ENV_STRENGTH: u32 = 13u;
const P_ENV_ROT: u32 = 14u;
const P_N_NODES: u32 = 15u;
const P_CAM_POS: u32 = 16u;
const P_TAN_HALF: u32 = 19u;
const P_CAM_R: u32 = 20u;
const P_ASPECT: u32 = 23u;
const P_CAM_U: u32 = 24u;
const P_CAM_F: u32 = 28u;
const P_ENV_C0: u32 = 32u;
const P_ENV_C1: u32 = 36u;
const P_ENV_C2: u32 = 40u;
const P_OFF_NODES: u32 = 44u;
const P_OFF_TRIS: u32 = 45u;
const P_OFF_PRIMS: u32 = 46u;
const P_OFF_MATS: u32 = 47u;
const P_OFF_TEXINFO: u32 = 48u;
const P_OFF_TEXPIX: u32 = 49u;
const P_OFF_LIGHTS: u32 = 50u;
const P_OFF_ENVPIX: u32 = 51u;
const P_OFF_ENV_MFUNC: u32 = 52u;
const P_OFF_ENV_MCDF: u32 = 53u;
const P_OFF_ENV_CFUNC: u32 = 54u;
const P_OFF_ENV_CCDF: u32 = 55u;
const P_OFF_ENV_CINT: u32 = 56u;
const P_OFF_EM_PRIMS: u32 = 57u;
const P_OFF_EM_FUNC: u32 = 58u;
const P_OFF_EM_CDF: u32 = 59u;
const P_OFF_EM_AREA: u32 = 60u;
const P_OFF_MSLUT: u32 = 61u;
const P_ENV_W: u32 = 62u;
const P_ENV_H: u32 = 63u;
const P_ENV_MINT: u32 = 64u;
const P_EM_INTEGRAL: u32 = 65u;
const P_N_SELECT: u32 = 66u;
const P_OFF_PORTALS: u32 = 76u;
const P_N_PORTALS: u32 = 77u;
const P_PORTAL_AREA: u32 = 78u;
const P_TILES_X: u32 = 79u;
const ADAPTIVE_TILE: u32 = 16u;
const PORTAL_WORDS: u32 = 16u;

const TRI_WORDS: u32 = 12u;
const PRIM_WORDS: u32 = 28u;
const MAT_WORDS: u32 = 36u;
const LIGHT_WORDS: u32 = 24u;
const ROUGH_GLASS: f32 = 0.03;

fn pu(i: u32) -> u32 { return P[i >> 2u][i & 3u]; }
fn pf(i: u32) -> f32 { return bitcast<f32>(pu(i)); }
fn pv3(i: u32) -> vec3<f32> { return vec3<f32>(pf(i), pf(i + 1u), pf(i + 2u)); }
fn sf(i: u32) -> f32 { return bitcast<f32>(S[i]); }
fn sv3(i: u32) -> vec3<f32> { return vec3<f32>(sf(i), sf(i + 1u), sf(i + 2u)); }

// ---------------------------------------------------------------- RNG
var<private> rng: u32;

fn pcg_hash(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn rnd() -> f32 {
    rng = rng * 747796405u + 2891336453u;
    let s = rng;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    let r = (w >> 22u) ^ w;
    return f32(r >> 8u) * (1.0 / 16777216.0);
}

fn luminance(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)); }

// ---------------------------------------------------------------- Geometria
struct Hit { t: f32, prim: u32, u: f32, v: f32, found: bool };

fn tri_intersect(slot: u32, o: vec3<f32>, d: vec3<f32>, tmin: f32, tmax: f32) -> vec3<f32> {
    // Restituisce (t, u, v), t < 0 se nessuna intersezione.
    let b = pu(P_OFF_TRIS) + slot * TRI_WORDS;
    let v0 = sv3(b);
    let e1 = sv3(b + 4u);
    let e2 = sv3(b + 8u);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if (abs(det) < 1e-20) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let inv = 1.0 / det;
    let s = o - v0;
    let u = dot(s, p) * inv;
    if (u < 0.0 || u > 1.0) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if (v < 0.0 || u + v > 1.0) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let t = dot(e2, q) * inv;
    if (t > tmin && t < tmax) { return vec3<f32>(t, u, v); }
    return vec3<f32>(-1.0, 0.0, 0.0);
}

fn aabb_hit(node: u32, o: vec3<f32>, inv: vec3<f32>, tmin: f32, tmax: f32) -> f32 {
    let b = pu(P_OFF_NODES) + node * 8u;
    let t0 = (sv3(b) - o) * inv;
    let t1 = (sv3(b + 4u) - o) * inv;
    let tn = min(t0, t1);
    let tf = max(t0, t1);
    let enter = max(max(max(tn.x, tn.y), tn.z), tmin);
    let exit = min(min(min(tf.x, tf.y), tf.z), tmax);
    if (enter <= exit) { return enter; }
    return -1.0;
}

// Attraversamento della BVH. `any_hit`: si ferma alla prima intersezione (ombre).
fn trace(o: vec3<f32>, d: vec3<f32>, tmin: f32, tmax_in: f32, any_hit: bool) -> Hit {
    var h: Hit;
    h.found = false;
    h.t = tmax_in;
    if (pu(P_N_NODES) == 0u) { return h; }
    let inv = 1.0 / d;
    var stack: array<u32, 64>;
    var sp: u32 = 0u;
    if (aabb_hit(0u, o, inv, tmin, h.t) < 0.0) { return h; }
    var node: u32 = 0u;
    loop {
        let b = pu(P_OFF_NODES) + node * 8u;
        let count = S[b + 7u];
        let lf = S[b + 3u];
        var descend = false;
        if (count > 0u) {
            for (var i = lf; i < lf + count; i = i + 1u) {
                let r = tri_intersect(i, o, d, tmin, h.t);
                if (r.x > 0.0) {
                    let prim = S[pu(P_OFF_TRIS) + i * TRI_WORDS + 3u];
                    if (pu(P_HAS_CUTOUT) == 0u || opaque_at(prim, r.x, r.y, r.z, o, d)) {
                        h.t = r.x;
                        h.u = r.y;
                        h.v = r.z;
                        h.prim = prim;
                        h.found = true;
                        if (any_hit) { return h; }
                    }
                }
            }
        } else {
            let tl = aabb_hit(lf, o, inv, tmin, h.t);
            let tr = aabb_hit(lf + 1u, o, inv, tmin, h.t);
            if (tl >= 0.0 && tr >= 0.0) {
                if (tl <= tr) { stack[sp] = lf + 1u; node = lf; } else { stack[sp] = lf; node = lf + 1u; }
                sp = sp + 1u;
                descend = true;
            } else if (tl >= 0.0) {
                node = lf;
                descend = true;
            } else if (tr >= 0.0) {
                node = lf + 1u;
                descend = true;
            }
        }
        if (descend) { continue; }
        var found_next = false;
        loop {
            if (sp == 0u) { break; }
            sp = sp - 1u;
            let n = stack[sp];
            if (aabb_hit(n, o, inv, tmin, h.t) >= 0.0) {
                node = n;
                found_next = true;
                break;
            }
        }
        if (!found_next) { break; }
    }
    return h;
}

// Wächter & Binder, "A Fast and Robust Method for Avoiding Self-Intersection".
// Componente per componente: niente indici dinamici (il compilatore FXC di DX12 li gestisce male).
fn offset_1(p: f32, n: f32) -> f32 {
    if (abs(p) < 1.0 / 32.0) { return p + (1.0 / 65536.0) * n; }
    var delta = i32(256.0 * n);
    if (p < 0.0) { delta = -delta; }
    return bitcast<f32>(bitcast<i32>(p) + delta);
}

fn offset_ray(p: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(offset_1(p.x, n.x), offset_1(p.y, n.y), offset_1(p.z, n.z));
}

struct Onb { t: vec3<f32>, b: vec3<f32>, n: vec3<f32> };

fn onb(n: vec3<f32>) -> Onb {
    var sign = 1.0;
    if (n.z < 0.0) { sign = -1.0; }
    let a = -1.0 / (sign + n.z);
    let b = n.x * n.y * a;
    var o: Onb;
    o.t = vec3<f32>(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x);
    o.b = vec3<f32>(b, sign + n.y * n.y * a, -n.y);
    o.n = n;
    return o;
}
fn to_local(f: Onb, v: vec3<f32>) -> vec3<f32> { return vec3<f32>(dot(v, f.t), dot(v, f.b), dot(v, f.n)); }
fn to_world(f: Onb, v: vec3<f32>) -> vec3<f32> { return f.t * v.x + f.b * v.y + f.n * v.z; }

// ---------------------------------------------------------------- Campionamento
fn cosine_hemisphere(u1: f32, u2: f32) -> vec3<f32> {
    let ox = 2.0 * u1 - 1.0;
    let oy = 2.0 * u2 - 1.0;
    var d = vec2<f32>(0.0, 0.0);
    if (ox != 0.0 || oy != 0.0) {
        var r: f32;
        var th: f32;
        if (abs(ox) > abs(oy)) { r = ox; th = (PI / 4.0) * (oy / ox); } else { r = oy; th = PI / 2.0 - (PI / 4.0) * (ox / oy); }
        d = vec2<f32>(r * cos(th), r * sin(th));
    }
    return vec3<f32>(d.x, d.y, sqrt(max(1.0 - d.x * d.x - d.y * d.y, 0.0)));
}

fn power_heuristic(a: f32, b: f32) -> f32 {
    let a2 = a * a;
    let b2 = b * b;
    if (a2 + b2 == 0.0) { return 0.0; }
    return a2 / (a2 + b2);
}

// Distribuzione 1D costante a tratti (come Distribution1D in Rust).
// Restituisce (x continuo, pdf densità, indice).
fn dist_sample(func_off: u32, cdf_off: u32, n: u32, integral: f32, u: f32) -> vec3<f32> {
    // Ultimo indice con cdf[i] <= u (ricerca binaria su n+1 valori).
    var lo: u32 = 0u;
    var hi: u32 = n;
    loop {
        if (lo >= hi) { break; }
        let mid = (lo + hi + 1u) / 2u;
        if (sf(cdf_off + mid) <= u) { lo = mid; } else { hi = mid - 1u; }
    }
    let idx = min(lo, n - 1u);
    let c0 = sf(cdf_off + idx);
    let width = sf(cdf_off + idx + 1u) - c0;
    var du = 0.0;
    if (width > 0.0) { du = (u - c0) / width; }
    var pdf = 1.0;
    if (integral > 0.0) { pdf = sf(func_off + idx) / integral; }
    return vec3<f32>(min((f32(idx) + du) / f32(n), 0.99999994), pdf, f32(idx));
}

// ---------------------------------------------------------------- Materiali
struct Mat {
    base: vec3<f32>, metallic: f32, alpha: f32, ior: f32, f0d: f32, spec_scale: f32,
    transmission: f32, roughness: f32, emission: vec3<f32>,
    tex_base: i32, tex_rough: i32, tex_normal: i32, normal_strength: f32,
    proj_box: bool, map_scale: f32, map_sin: f32, map_cos: f32, map_off: vec2<f32>,
    tex_metal: i32, tex_height: i32, tex_ao: i32, tex_opacity: i32,
    normal_dx: bool, gloss: bool, bump_m: f32, ao_strength: f32,
};

fn has_textures(m: Mat) -> bool {
    return m.tex_base >= 0 || m.tex_rough >= 0 || m.tex_normal >= 0 || m.tex_metal >= 0 || m.tex_height >= 0 || m.tex_ao >= 0;
}

fn load_mat(i: u32) -> Mat {
    let b = pu(P_OFF_MATS) + i * MAT_WORDS;
    var m: Mat;
    m.base = sv3(b);
    m.metallic = sf(b + 3u);
    m.alpha = sf(b + 4u);
    m.ior = sf(b + 5u);
    m.f0d = sf(b + 6u);
    m.spec_scale = sf(b + 7u);
    m.transmission = sf(b + 8u);
    m.roughness = sf(b + 9u);
    m.emission = sv3(b + 10u);
    m.tex_base = bitcast<i32>(S[b + 13u]);
    m.tex_rough = bitcast<i32>(S[b + 14u]);
    m.tex_normal = bitcast<i32>(S[b + 15u]);
    m.normal_strength = sf(b + 16u);
    m.proj_box = S[b + 17u] == 1u;
    m.map_scale = sf(b + 18u);
    m.map_sin = sf(b + 19u);
    m.map_cos = sf(b + 20u);
    m.map_off = vec2<f32>(sf(b + 21u), sf(b + 22u));
    m.tex_metal = bitcast<i32>(S[b + 23u]);
    m.tex_height = bitcast<i32>(S[b + 24u]);
    m.tex_ao = bitcast<i32>(S[b + 25u]);
    m.tex_opacity = bitcast<i32>(S[b + 26u]);
    m.normal_dx = (S[b + 27u] & 1u) != 0u;
    m.gloss = (S[b + 27u] & 2u) != 0u;
    m.bump_m = sf(b + 28u);
    m.ao_strength = sf(b + 29u);
    return m;
}

fn srgb_to_linear(c: f32) -> f32 {
    if (c <= 0.04045) { return c / 12.92; }
    return pow((c + 0.055) / 1.055, 2.4);
}

fn texel(tex: u32, x: i32, y: i32) -> vec3<f32> {
    let ib = pu(P_OFF_TEXINFO) + tex * 4u;
    let w = i32(S[ib + 1u]);
    let h = i32(S[ib + 2u]);
    let xx = ((x % w) + w) % w;
    let yy = ((y % h) + h) % h;
    let px = unpack4x8unorm(S[pu(P_OFF_TEXPIX) + S[ib] + u32(yy * w + xx)]).xyz;
    if (S[ib + 3u] == 1u) { return vec3<f32>(srgb_to_linear(px.x), srgb_to_linear(px.y), srgb_to_linear(px.z)); }
    return px;
}

fn tex_sample(tex: u32, uv: vec2<f32>) -> vec3<f32> {
    let ib = pu(P_OFF_TEXINFO) + tex * 4u;
    let w = f32(S[ib + 1u]);
    let h = f32(S[ib + 2u]);
    let fx = uv.x * w - 0.5;
    let fy = (1.0 - uv.y) * h - 0.5;
    let x0 = floor(fx);
    let y0 = floor(fy);
    let tx = fx - x0;
    let ty = fy - y0;
    let xi = i32(x0);
    let yi = i32(y0);
    let top = mix(texel(tex, xi, yi), texel(tex, xi + 1, yi), tx);
    let bot = mix(texel(tex, xi, yi + 1), texel(tex, xi + 1, yi + 1), tx);
    return mix(top, bot, ty);
}

struct TexCoords { uv: vec2<f32>, du: vec3<f32>, dv: vec3<f32>, m_per_uv: f32 };

// Coordinate di texture e direzioni tangenti (come forge_render_cpu::textures::coordinates).
fn tex_coords(m: Mat, prim: u32, bu: f32, bv: f32, p: vec3<f32>) -> TexCoords {
    let pb = pu(P_OFF_PRIMS) + prim * PRIM_WORDS;
    let tangent = sv3(pb + 22u);
    let ngeo = sv3(pb + 12u);
    let use_uv = !m.proj_box && dot(tangent, tangent) > 0.0;
    var base: vec2<f32>;
    var t: vec3<f32>;
    var b: vec3<f32>;
    var is_box = false;
    let flags = S[pb + 27u];
    if ((flags & 1u) != 0u) {
        // Superficie spostata dal displacement: coordinate box della superficie originale.
        is_box = true;
        let a0 = vec2<f32>(sf(pb + 16u), sf(pb + 17u));
        let a1 = vec2<f32>(sf(pb + 18u), sf(pb + 19u));
        let a2 = vec2<f32>(sf(pb + 20u), sf(pb + 21u));
        base = a0 * (1.0 - bu - bv) + a1 * bu + a2 * bv;
        let axis = (flags >> 1u) & 3u;
        if (axis == 0u) { t = vec3<f32>(0.0, 0.0, 1.0); b = vec3<f32>(0.0, 1.0, 0.0); }
        else if (axis == 1u) { t = vec3<f32>(1.0, 0.0, 0.0); b = vec3<f32>(0.0, 0.0, 1.0); }
        else { t = vec3<f32>(1.0, 0.0, 0.0); b = vec3<f32>(0.0, 1.0, 0.0); }
    } else if (use_uv) {
        let a0 = vec2<f32>(sf(pb + 16u), sf(pb + 17u));
        let a1 = vec2<f32>(sf(pb + 18u), sf(pb + 19u));
        let a2 = vec2<f32>(sf(pb + 20u), sf(pb + 21u));
        base = a0 * (1.0 - bu - bv) + a1 * bu + a2 * bv;
        t = normalize(tangent);
        b = cross(ngeo, t);
    } else {
        is_box = true;
        let n = abs(ngeo);
        if (n.x >= n.y && n.x >= n.z) { base = vec2<f32>(p.z, p.y); t = vec3<f32>(0.0, 0.0, 1.0); b = vec3<f32>(0.0, 1.0, 0.0); }
        else if (n.y >= n.z) { base = vec2<f32>(p.x, p.z); t = vec3<f32>(1.0, 0.0, 0.0); b = vec3<f32>(0.0, 0.0, 1.0); }
        else { base = vec2<f32>(p.x, p.y); t = vec3<f32>(1.0, 0.0, 0.0); b = vec3<f32>(0.0, 1.0, 0.0); }
    }
    var s = m.map_scale;
    var mpu = 1.0;
    if (is_box) {
        s = 1.0 / max(m.map_scale, 1e-6);
        mpu = max(m.map_scale, 1e-6);
    }
    let q = base * s;
    let sn = m.map_sin;
    let cs = m.map_cos;
    var r: TexCoords;
    r.uv = vec2<f32>(q.x * cs - q.y * sn, q.x * sn + q.y * cs) + m.map_off;
    r.du = t * cs - b * sn;
    r.dv = t * sn + b * cs;
    r.m_per_uv = mpu;
    return r;
}

// Riferimento tangente ortonormale attorno a n, orientato come (du, dv); w = 0 se degenere.
fn tangent_t(n: vec3<f32>, du: vec3<f32>) -> vec4<f32> {
    let tt0 = du - n * dot(du, n);
    if (dot(tt0, tt0) <= 0.0) { return vec4<f32>(0.0); }
    return vec4<f32>(normalize(tt0), 1.0);
}

// Applica le texture (come forge_render_cpu::textures::apply).
fn apply_textures(m: ptr<function, Mat>, prim: u32, bu: f32, bv: f32, p: vec3<f32>, ns: ptr<function, vec3<f32>>) {
    let tc = tex_coords(*m, prim, bu, bv, p);
    let uv = tc.uv;
    if ((*m).tex_base >= 0) { (*m).base = (*m).base * tex_sample(u32((*m).tex_base), uv); }
    if ((*m).tex_ao >= 0) {
        let ao = tex_sample(u32((*m).tex_ao), uv).x;
        (*m).base = (*m).base * (1.0 + (ao - 1.0) * (*m).ao_strength);
    }
    if ((*m).tex_rough >= 0) {
        var rr = tex_sample(u32((*m).tex_rough), uv).x;
        if ((*m).gloss) { rr = 1.0 - rr; }
        let r = clamp(rr * (*m).roughness, 0.0, 1.0);
        (*m).alpha = max(r * r, 1e-3);
    }
    if ((*m).tex_metal >= 0) {
        (*m).metallic = clamp((*m).metallic * tex_sample(u32((*m).tex_metal), uv).x, 0.0, 1.0);
    }
    if ((*m).tex_normal >= 0) {
        var c = tex_sample(u32((*m).tex_normal), uv) * 2.0 - vec3<f32>(1.0);
        if ((*m).normal_dx) { c.y = -c.y; }
        let n = *ns;
        let tf = tangent_t(n, tc.du);
        if (tf.w > 0.0) {
            let tt = tf.xyz;
            var bb = cross(n, tt);
            if (dot(bb, tc.dv) < 0.0) { bb = -bb; }
            let pert = tt * (c.x * (*m).normal_strength) + bb * (c.y * (*m).normal_strength) + n * max(c.z, 1e-3);
            if (dot(pert, pert) > 0.0) { *ns = normalize(pert); }
        }
    }
    if ((*m).tex_height >= 0 && (*m).bump_m > 0.0) {
        // Bump mapping: gradiente dell'altezza con differenze centrate di un texel.
        let ht = u32((*m).tex_height);
        let ib = pu(P_OFF_TEXINFO) + ht * 4u;
        let eu = 1.0 / f32(max(S[ib + 1u], 1u));
        let ev = 1.0 / f32(max(S[ib + 2u], 1u));
        let gu = (tex_sample(ht, uv + vec2<f32>(eu, 0.0)).x - tex_sample(ht, uv - vec2<f32>(eu, 0.0)).x) / (2.0 * eu);
        let gv = (tex_sample(ht, uv + vec2<f32>(0.0, ev)).x - tex_sample(ht, uv - vec2<f32>(0.0, ev)).x) / (2.0 * ev);
        let n = *ns;
        let tf = tangent_t(n, tc.du);
        if (tf.w > 0.0) {
            let tt = tf.xyz;
            var bb = cross(n, tt);
            if (dot(bb, tc.dv) < 0.0) { bb = -bb; }
            let k = (*m).bump_m / tc.m_per_uv;
            let pert = n - tt * (gu * k) - bb * (gv * k);
            if (dot(pert, pert) > 0.0) { *ns = normalize(pert); }
        }
    }
}

// Ritaglio: vero se il punto del triangolo è opaco (come forge_render_cpu::textures::opaque_at).
fn opaque_at(prim: u32, t: f32, bu: f32, bv: f32, o: vec3<f32>, d: vec3<f32>) -> bool {
    let pb = pu(P_OFF_PRIMS) + prim * PRIM_WORDS;
    let mb = pu(P_OFF_MATS) + S[pb + 3u] * MAT_WORDS;
    let top = bitcast<i32>(S[mb + 26u]);
    if (top < 0) { return true; }
    let m = load_mat(S[pb + 3u]);
    let tc = tex_coords(m, prim, bu, bv, o + d * t);
    return tex_sample(u32(top), tc.uv).x >= 0.5;
}

// ---------------------------------------------------------------- BSDF
fn schlick(f0: f32, c: f32) -> f32 {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    return f0 + (1.0 - f0) * m2 * m2 * m;
}
fn schlick3(f0: vec3<f32>, c: f32) -> vec3<f32> {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    return f0 + (vec3<f32>(1.0) - f0) * (m2 * m2 * m);
}
// (F, cos θt); eta = n_t / n_i.
fn fresnel_dielectric(cos_i_in: f32, eta: f32) -> vec2<f32> {
    let cos_i = clamp(cos_i_in, 0.0, 1.0);
    let sin2_t = (1.0 - cos_i * cos_i) / (eta * eta);
    if (sin2_t >= 1.0) { return vec2<f32>(1.0, 0.0); }
    let cos_t = sqrt(1.0 - sin2_t);
    let rs = (cos_i - eta * cos_t) / (cos_i + eta * cos_t);
    let rp = (eta * cos_i - cos_t) / (eta * cos_i + cos_t);
    return vec2<f32>(0.5 * (rs * rs + rp * rp), cos_t);
}
fn ggx_d(h: vec3<f32>, a2: f32) -> f32 {
    let c2 = h.z * h.z;
    let d = c2 * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}
fn smith_lambda(w: vec3<f32>, a2: f32) -> f32 {
    let c2 = w.z * w.z;
    if (c2 <= 0.0) { return INF; }
    let t2 = max(1.0 - c2, 0.0) / c2;
    return 0.5 * (sqrt(1.0 + a2 * t2) - 1.0);
}
fn sample_vndf(wo: vec3<f32>, alpha: f32, u1: f32, u2: f32) -> vec3<f32> {
    let vh = normalize(vec3<f32>(alpha * wo.x, alpha * wo.y, wo.z));
    let lensq = vh.x * vh.x + vh.y * vh.y;
    var t1 = vec3<f32>(1.0, 0.0, 0.0);
    if (lensq > 0.0) { t1 = vec3<f32>(-vh.y, vh.x, 0.0) / sqrt(lensq); }
    let t2 = cross(vh, t1);
    let r = sqrt(u1);
    let phi = TAU * u2;
    let p1 = r * cos(phi);
    var p2 = r * sin(phi);
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * sqrt(max(1.0 - p1 * p1, 0.0)) + s * p2;
    let nh = t1 * p1 + t2 * p2 + vh * sqrt(max(1.0 - p1 * p1 - p2 * p2, 0.0));
    return normalize(vec3<f32>(alpha * nh.x, alpha * nh.y, max(nh.z, 1e-6)));
}
fn ggx_albedo(cos_o: f32, alpha: f32) -> f32 {
    let n = 32.0;
    let fa = min(clamp(alpha, 0.0, 1.0) * (n - 1.0), n - 1.0 - 1e-4);
    let fc = min(clamp(cos_o, 0.0, 1.0) * (n - 1.0), n - 1.0 - 1e-4);
    let ia = u32(fa);
    let ic = u32(fc);
    let ta = fa - f32(ia);
    let tc = fc - f32(ic);
    let o = pu(P_OFF_MSLUT);
    let top = sf(o + ia * 32u + ic) * (1.0 - tc) + sf(o + ia * 32u + ic + 1u) * tc;
    let bot = sf(o + (ia + 1u) * 32u + ic) * (1.0 - tc) + sf(o + (ia + 1u) * 32u + ic + 1u) * tc;
    return max(top * (1.0 - ta) + bot * ta, 1e-3);
}

struct Bsdf { frame: Onb, ng: vec3<f32>, m: Mat, entering: bool };

fn f_diel(bs: Bsdf, c: f32) -> f32 { return min(bs.m.spec_scale * schlick(bs.m.f0d, c), 1.0); }
fn f0_of(bs: Bsdf) -> vec3<f32> { return mix(vec3<f32>(min(bs.m.spec_scale * bs.m.f0d, 1.0)), bs.m.base, bs.m.metallic); }
fn rough_glass(bs: Bsdf) -> bool { return bs.m.transmission > 0.0 && bs.m.roughness >= ROUGH_GLASS; }
fn has_non_delta(bs: Bsdf) -> bool { return bs.m.transmission < 1.0 || rough_glass(bs); }
fn ior_pair(bs: Bsdf) -> vec2<f32> {
    if (bs.entering) { return vec2<f32>(1.0, bs.m.ior); }
    return vec2<f32>(bs.m.ior, 1.0);
}
fn spec_prob(bs: Bsdf, wo: vec3<f32>) -> f32 {
    var floor_v = 0.0;
    if (bs.m.metallic > 0.0 || bs.m.spec_scale > 0.0) { floor_v = 1e-3; }
    let spec = max(luminance(schlick3(f0_of(bs), wo.z)), floor_v);
    let diff = (1.0 - bs.m.metallic) * luminance(bs.m.base) * (1.0 - f_diel(bs, wo.z));
    if (diff <= 0.0) { return 1.0; }
    if (spec <= 0.0) { return 0.0; }
    return clamp(spec / (spec + diff), 0.1, 0.9);
}

// (f.rgb, pdf)
fn eval_opaque(bs: Bsdf, wo: vec3<f32>, wi: vec3<f32>) -> vec4<f32> {
    if (wo.z <= 0.0 || wi.z <= 0.0) { return vec4<f32>(0.0); }
    let m = bs.m;
    let ps = spec_prob(bs, wo);
    let a2 = m.alpha * m.alpha;
    let h = normalize(wo + wi);
    var f = vec3<f32>(0.0);
    var pdf = 0.0;
    if (ps > 0.0) {
        let d = ggx_d(h, a2);
        let lo = smith_lambda(wo, a2);
        let li = smith_lambda(wi, a2);
        let g2 = 1.0 / (1.0 + lo + li);
        let g1o = 1.0 / (1.0 + lo);
        let voh = max(dot(wo, h), 0.0);
        let fr = mix(vec3<f32>(f_diel(bs, voh)), schlick3(m.base, voh), m.metallic);
        let e = ggx_albedo(wo.z, m.alpha);
        let ms = vec3<f32>(1.0) + f0_of(bs) * (1.0 / e - 1.0);
        f = f + fr * ms * (d * g2 / (4.0 * wo.z * wi.z));
        pdf = pdf + ps * g1o * d / (4.0 * wo.z);
    }
    if (ps < 1.0) {
        let kd = (1.0 - m.metallic) * (1.0 - f_diel(bs, wi.z)) * (1.0 - f_diel(bs, wo.z));
        f = f + m.base * (kd * INV_PI);
        pdf = pdf + (1.0 - ps) * max(wi.z, 0.0) * INV_PI;
    }
    return vec4<f32>(f, pdf);
}

fn eval_rough_glass(bs: Bsdf, wo: vec3<f32>, wi: vec3<f32>) -> vec4<f32> {
    if (wo.z <= 0.0 || wi.z == 0.0) { return vec4<f32>(0.0); }
    let m = bs.m;
    let ip = ior_pair(bs);
    let eta = ip.y / ip.x;
    let refl = wi.z > 0.0;
    var wm: vec3<f32>;
    if (refl) { wm = wi + wo; } else { wm = wi * eta + wo; }
    if (dot(wm, wm) == 0.0) { return vec4<f32>(0.0); }
    wm = normalize(wm);
    if (wm.z < 0.0) { wm = -wm; }
    let wom = dot(wo, wm);
    let wim = dot(wi, wm);
    if (wom * wo.z <= 0.0 || wim * wi.z <= 0.0) { return vec4<f32>(0.0); }
    let a2 = m.alpha * m.alpha;
    let fr = fresnel_dielectric(wom, eta).x;
    let d = ggx_d(wm, a2);
    let lo = smith_lambda(wo, a2);
    let li = smith_lambda(vec3<f32>(wi.x, wi.y, abs(wi.z)), a2);
    let g2 = 1.0 / (1.0 + lo + li);
    let g1o = 1.0 / (1.0 + lo);
    let dv = g1o * wom * d / wo.z;
    if (refl) {
        let f = d * g2 * fr / (4.0 * wo.z * wi.z);
        return vec4<f32>(vec3<f32>(f), dv / (4.0 * wom) * fr);
    }
    let denom = (wim + wom / eta) * (wim + wom / eta);
    if (denom <= 0.0) { return vec4<f32>(0.0); }
    let ft = (1.0 - fr) * d * g2 * abs(wim * wom / (wi.z * wo.z * denom)) / (eta * eta);
    let pdf = dv * (abs(wim) / denom) * (1.0 - fr);
    return vec4<f32>(m.base * ft, pdf);
}

fn bsdf_eval(bs: Bsdf, wo_w: vec3<f32>, wi_w: vec3<f32>) -> vec4<f32> {
    if (dot(wo_w, bs.ng) <= 0.0) { return vec4<f32>(0.0); }
    let wo = to_local(bs.frame, wo_w);
    let wi = to_local(bs.frame, wi_w);
    let geo_refl = dot(wi_w, bs.ng) > 0.0;
    if (geo_refl != (wi.z > 0.0)) { return vec4<f32>(0.0); }
    let t = bs.m.transmission;
    var r = vec4<f32>(0.0);
    if (t < 1.0 && geo_refl) { r = r + eval_opaque(bs, wo, wi) * (1.0 - t); }
    if (rough_glass(bs)) { r = r + eval_rough_glass(bs, wo, wi) * t; }
    return r;
}

struct BSample { wi: vec3<f32>, weight: vec3<f32>, pdf: f32, delta: bool, ok: bool };

fn bsdf_sample(bs: Bsdf, wo_w: vec3<f32>, u_lobe: f32, u1: f32, u2: f32) -> BSample {
    var s: BSample;
    s.ok = false;
    s.delta = false;
    let m = bs.m;
    let wo = to_local(bs.frame, wo_w);
    if (wo.z <= 0.0) { return s; }
    var wi: vec3<f32>;
    if (u_lobe < m.transmission) {
        let ip = ior_pair(bs);
        if (!rough_glass(bs)) {
            // Vetro liscio (delta).
            let n = bs.frame.n;
            let cos_o = dot(wo_w, n);
            if (cos_o <= 0.0) { return s; }
            let fc = fresnel_dielectric(cos_o, ip.y / ip.x);
            s.delta = true;
            s.pdf = 0.0;
            s.ok = true;
            if (u1 < fc.x) {
                s.wi = 2.0 * cos_o * n - wo_w;
                s.weight = vec3<f32>(1.0);
                return s;
            }
            let eta = ip.x / ip.y;
            s.wi = normalize(-wo_w * eta + n * (eta * cos_o - fc.y));
            s.weight = m.base * (eta * eta);
            return s;
        }
        let eta = ip.y / ip.x;
        let wm = sample_vndf(wo, m.alpha, u1, u2);
        let wom = dot(wo, wm);
        let fc = fresnel_dielectric(wom, eta);
        let u3 = u_lobe / m.transmission;
        if (u3 < fc.x) { wi = 2.0 * wom * wm - wo; } else { wi = -wo / eta + wm * (wom / eta - fc.y); }
    } else {
        var u = u_lobe;
        if (m.transmission > 0.0) { u = (u_lobe - m.transmission) / (1.0 - m.transmission); }
        if (u < spec_prob(bs, wo)) {
            let h = sample_vndf(wo, m.alpha, u1, u2);
            wi = 2.0 * dot(wo, h) * h - wo;
        } else {
            wi = cosine_hemisphere(u1, u2);
        }
    }
    if (wi.z == 0.0 || dot(wi, wi) == 0.0) { return s; }
    let wi_w = to_world(bs.frame, normalize(wi));
    let e = bsdf_eval(bs, wo_w, wi_w);
    if (e.w <= 0.0 || (e.x == 0.0 && e.y == 0.0 && e.z == 0.0)) { return s; }
    s.wi = wi_w;
    s.weight = e.xyz * (abs(dot(wi_w, bs.frame.n)) / e.w);
    s.pdf = e.w;
    s.ok = true;
    return s;
}

// ---------------------------------------------------------------- Luci e ambiente
struct LSample { wi: vec3<f32>, dist: f32, li: vec3<f32>, pdf: f32, delta: bool, ok: bool, shadows: bool };

fn light_base(i: u32) -> u32 { return pu(P_OFF_LIGHTS) + i * LIGHT_WORDS; }

fn sample_light(i: u32, p: vec3<f32>, u1: f32, u2: f32) -> LSample {
    var s: LSample;
    s.ok = false;
    let b = light_base(i);
    let kind = S[b];
    s.shadows = S[b + 1u] == 1u;
    let em = sv3(b + 4u);
    let a = sv3(b + 8u);
    let dir = sv3(b + 12u);
    if (kind == 2u) {
        s.wi = -dir;
        s.dist = INF;
        s.li = em;
        s.pdf = 0.0;
        s.delta = true;
        s.ok = true;
        return s;
    }
    if (kind == 3u) {
        let hw = sf(b + 7u);
        let hh = sf(b + 11u);
        let area = sf(b + 15u);
        let q = a + sv3(b + 16u) * ((2.0 * u1 - 1.0) * hw) + sv3(b + 20u) * ((2.0 * u2 - 1.0) * hh);
        let d = q - p;
        let d2 = dot(d, d);
        if (d2 <= 0.0) { return s; }
        let dist = sqrt(d2);
        let wi = d / dist;
        let cos_l = dot(-wi, dir);
        if (cos_l <= 0.0) { return s; }
        s.wi = wi;
        s.dist = dist;
        s.li = em;
        s.pdf = d2 / (cos_l * area);
        s.delta = false;
        s.ok = true;
        return s;
    }
    let d = a - p;
    let d2 = dot(d, d);
    if (d2 <= 0.0) { return s; }
    let dist = sqrt(d2);
    let wi = d / dist;
    var fall = 1.0;
    if (kind == 1u) {
        let ci = sf(b + 19u);
        let co = sf(b + 23u);
        let c = dot(-wi, dir);
        let t = clamp((c - co) / (max(ci, co + 1e-4) - co), 0.0, 1.0);
        fall = t * t * (3.0 - 2.0 * t);
        if (fall <= 0.0) { return s; }
    }
    s.wi = wi;
    s.dist = dist;
    s.li = em * (fall / d2);
    s.pdf = 0.0;
    s.delta = true;
    s.ok = true;
    return s;
}

// (t, pdf solido, colpita frontale) per una luce ad area; t < 0 se mancata.
fn intersect_area(i: u32, o: vec3<f32>, d: vec3<f32>, tmax: f32) -> vec3<f32> {
    let b = light_base(i);
    if (S[b] != 3u) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let center = sv3(b + 8u);
    let n = sv3(b + 12u);
    let denom = dot(d, n);
    if (abs(denom) < 1e-12) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let t = dot(center - o, n) / denom;
    if (t <= 1e-6 || t >= tmax) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let local = o + d * t - center;
    if (abs(dot(local, sv3(b + 16u))) > sf(b + 7u) || abs(dot(local, sv3(b + 20u))) > sf(b + 11u)) { return vec3<f32>(-1.0, 0.0, 0.0); }
    let cos_l = -denom;
    if (cos_l <= 0.0) { return vec3<f32>(t, 0.0, 0.0); }
    return vec3<f32>(t, t * t / (cos_l * sf(b + 15u)), 1.0);
}

fn env_dir_to_uv(d: vec3<f32>, rot: f32) -> vec2<f32> {
    let u = fract(0.5 + atan2(d.x, -d.z) / TAU - rot / TAU);
    let v = acos(clamp(d.y, -1.0, 1.0)) / PI;
    return vec2<f32>(u, v);
}
fn env_texel(x: u32, y: u32) -> vec3<f32> { return sv3(pu(P_OFF_ENVPIX) + (y * pu(P_ENV_W) + x) * 3u); }

fn env_radiance(d: vec3<f32>, smooth_lookup: bool) -> vec3<f32> {
    let kind = pu(P_ENV_KIND);
    let strength = pf(P_ENV_STRENGTH);
    if (kind == 0u) { return pv3(P_ENV_C0) * strength; }
    if (kind == 1u) {
        let y = clamp(d.y, -1.0, 1.0);
        if (y >= 0.0) { return mix(pv3(P_ENV_C1), pv3(P_ENV_C0), pow(y, 0.6)) * strength; }
        return mix(pv3(P_ENV_C1), pv3(P_ENV_C2), pow(-y, 0.4)) * strength;
    }
    let w = pu(P_ENV_W);
    let h = pu(P_ENV_H);
    let uv = env_dir_to_uv(d, pf(P_ENV_ROT));
    if (!smooth_lookup) {
        let x = min(u32(uv.x * f32(w)), w - 1u);
        let y = min(u32(uv.y * f32(h)), h - 1u);
        return env_texel(x, y) * strength;
    }
    let fx = uv.x * f32(w) - 0.5;
    let fy = clamp(uv.y * f32(h) - 0.5, 0.0, f32(h - 1u));
    let x0 = floor(fx);
    let y0 = floor(fy);
    let tx = fx - x0;
    let ty = fy - y0;
    let xa = u32((i32(x0) % i32(w) + i32(w)) % i32(w));
    let xb = (xa + 1u) % w;
    let ya = u32(y0);
    let yb = min(ya + 1u, h - 1u);
    let top = mix(env_texel(xa, ya), env_texel(xb, ya), tx);
    let bot = mix(env_texel(xa, yb), env_texel(xb, yb), tx);
    return mix(top, bot, ty) * strength;
}

fn env_pdf(d: vec3<f32>) -> f32 {
    let w = pu(P_ENV_W);
    let h = pu(P_ENV_H);
    let uv = env_dir_to_uv(d, pf(P_ENV_ROT));
    let x = min(u32(uv.x * f32(w)), w - 1u);
    let y = min(u32(uv.y * f32(h)), h - 1u);
    let sin_d = sin(uv.y * PI);
    if (sin_d <= 0.0) { return 0.0; }
    let mint = pf(P_ENV_MINT);
    var pm = 1.0;
    if (mint > 0.0) { pm = sf(pu(P_OFF_ENV_MFUNC) + y) / mint; }
    let cint = sf(pu(P_OFF_ENV_CINT) + y);
    var pc = 1.0;
    if (cint > 0.0) { pc = sf(pu(P_OFF_ENV_CFUNC) + y * w + x) / cint; }
    return pm * pc / (2.0 * PI * PI * sin_d);
}

// ---------------------------------------------------------------- Portali di luce
// Finestre da cui entra l'ambiente: la NEE sceglie un punto uniforme sulla loro area
// (stesso algoritmo di `Portals` in forge_render_core).
fn portal_base(i: u32) -> u32 { return pu(P_OFF_PORTALS) + i * PORTAL_WORDS; }

// Distanza a cui il raggio esce dalla stanza attraverso il portale i (−1 se non lo attraversa).
fn portal_hit(i: u32, o: vec3<f32>, d: vec3<f32>) -> f32 {
    let b = portal_base(i);
    let c = sv3(b);
    let n = sv3(b + 4u);
    let dn = dot(d, n);
    if (dn >= -1e-8) { return -1.0; }
    let t = dot(c - o, n) / dn;
    if (t <= 0.0) { return -1.0; }
    let q = o + d * t - c;
    if (abs(dot(q, sv3(b + 8u))) > sf(b + 3u) || abs(dot(q, sv3(b + 12u))) > sf(b + 7u)) { return -1.0; }
    return t;
}

fn portal_pdf(o: vec3<f32>, d: vec3<f32>) -> f32 {
    let total = pf(P_PORTAL_AREA);
    if (total <= 0.0) { return 0.0; }
    var pdf = 0.0;
    for (var i: u32 = 0u; i < pu(P_N_PORTALS); i = i + 1u) {
        let t = portal_hit(i, o, d);
        if (t > 0.0) {
            let cosv = max(-dot(d, sv3(portal_base(i) + 4u)), 1e-8);
            pdf = pdf + t * t / (cosv * total);
        }
    }
    return pdf;
}

fn portal_sample(p: vec3<f32>, u0: f32, u1: f32, u2: f32) -> LSample {
    var s: LSample;
    s.ok = false;
    let n_p = pu(P_N_PORTALS);
    let total = pf(P_PORTAL_AREA);
    if (n_p == 0u || total <= 0.0) { return s; }
    var x = u0 * total;
    var pick = n_p - 1u;
    for (var i: u32 = 0u; i < n_p; i = i + 1u) {
        let b = portal_base(i);
        let a = 4.0 * sf(b + 3u) * sf(b + 7u);
        if (x < a) { pick = i; break; }
        x = x - a;
    }
    let b = portal_base(pick);
    let q = sv3(b) + sv3(b + 8u) * ((2.0 * u1 - 1.0) * sf(b + 3u)) + sv3(b + 12u) * ((2.0 * u2 - 1.0) * sf(b + 7u));
    let dv = q - p;
    let d2 = dot(dv, dv);
    if (d2 <= 1e-12) { return s; }
    let wi = dv / sqrt(d2);
    if (-dot(wi, sv3(b + 4u)) <= 1e-6) { return s; }
    let pdf = portal_pdf(p, wi);
    if (pdf <= 0.0) { return s; }
    s.wi = wi;
    s.li = env_radiance(wi, false);
    s.dist = INF;
    s.pdf = pdf;
    s.delta = false;
    s.shadows = true;
    s.ok = true;
    return s;
}

// NEE dell'ambiente: attraverso i portali se ci sono, altrimenti la HDRI.
fn env_light_sample(p: vec3<f32>, u1: f32, u2: f32) -> LSample {
    if (pu(P_N_PORTALS) > 0u) { return portal_sample(p, rnd(), u1, u2); }
    return env_sample(u1, u2);
}

fn env_light_pdf(o: vec3<f32>, d: vec3<f32>) -> f32 {
    if (pu(P_N_PORTALS) > 0u) { return portal_pdf(o, d); }
    return env_pdf(d);
}

fn env_sample(u1: f32, u2: f32) -> LSample {
    var s: LSample;
    s.ok = false;
    let w = pu(P_ENV_W);
    let h = pu(P_ENV_H);
    let mv = dist_sample(pu(P_OFF_ENV_MFUNC), pu(P_OFF_ENV_MCDF), h, pf(P_ENV_MINT), u1);
    let y = u32(mv.z);
    let cv = dist_sample(pu(P_OFF_ENV_CFUNC) + y * w, pu(P_OFF_ENV_CCDF) + y * (w + 1u), w, sf(pu(P_OFF_ENV_CINT) + y), u2);
    let theta = mv.x * PI;
    let st = sin(theta);
    if (st <= 0.0) { return s; }
    let pdf = cv.y * mv.y / (2.0 * PI * PI * st);
    if (pdf <= 0.0) { return s; }
    let phi = (cv.x - 0.5) * TAU + pf(P_ENV_ROT);
    s.wi = vec3<f32>(st * sin(phi), cos(theta), -st * cos(phi));
    s.li = env_texel(u32(cv.z), y) * pf(P_ENV_STRENGTH);
    s.dist = INF;
    s.pdf = pdf;
    s.delta = false;
    s.shadows = true;
    s.ok = true;
    return s;
}

fn prim_vertices(prim: u32) -> array<vec3<f32>, 3> {
    // I triangoli sono in ordine BVH: si cerca lo slot tramite le normali non serve;
    // gli emissivi memorizzano direttamente lo slot nel buffer prim (parola 26).
    let slot = S[pu(P_OFF_PRIMS) + prim * PRIM_WORDS + 26u];
    let b = pu(P_OFF_TRIS) + slot * TRI_WORDS;
    let v0 = sv3(b);
    return array<vec3<f32>, 3>(v0, v0 + sv3(b + 4u), v0 + sv3(b + 8u));
}

fn emissive_area_pdf(prim: u32) -> f32 {
    let ei = S[pu(P_OFF_PRIMS) + prim * PRIM_WORDS + 25u];
    if (ei == 0xffffffffu) { return 0.0; }
    let n = pu(P_N_EMISSIVE);
    let integral = pf(P_EM_INTEGRAL);
    var pd = 1.0;
    if (integral > 0.0) { pd = sf(pu(P_OFF_EM_FUNC) + ei) / integral; }
    return pd / f32(n) / sf(pu(P_OFF_EM_AREA) + ei);
}

fn emissive_sample(p: vec3<f32>, u0: f32, u1: f32, u2: f32) -> LSample {
    var s: LSample;
    s.ok = false;
    let n = pu(P_N_EMISSIVE);
    let ds = dist_sample(pu(P_OFF_EM_FUNC), pu(P_OFF_EM_CDF), n, pf(P_EM_INTEGRAL), u0);
    let prim = S[pu(P_OFF_EM_PRIMS) + u32(ds.z)];
    let v = prim_vertices(prim);
    let su = sqrt(u1);
    let q = v[0] * (1.0 - su) + v[1] * (su * (1.0 - u2)) + v[2] * (su * u2);
    let d = q - p;
    let d2 = dot(d, d);
    if (d2 <= 0.0) { return s; }
    let dist = sqrt(d2);
    let wi = d / dist;
    let pb = pu(P_OFF_PRIMS) + prim * PRIM_WORDS;
    let cos_l = abs(dot(sv3(pb + 12u), wi));
    if (cos_l <= 1e-8) { return s; }
    s.wi = wi;
    s.dist = dist;
    s.li = load_mat(S[pb + 3u]).emission;
    s.pdf = emissive_area_pdf(prim) * d2 / cos_l;
    s.delta = false;
    s.shadows = true;
    s.ok = true;
    return s;
}

// ---------------------------------------------------------------- Nebbie (come forge_render_cpu::medium)
fn fog_base(k: u32) -> u32 { return pu(P_OFF_FOGS) + k * FOG_WORDS; }
fn fog_res(b: u32) -> vec3<u32> { return vec3<u32>(S[b + 4u], S[b + 5u], S[b + 6u]); }

fn fog_texel(b: u32, res: vec3<u32>, i: vec3<i32>) -> f32 {
    let c = clamp(i, vec3<i32>(0), vec3<i32>(res) - vec3<i32>(1));
    let idx = (u32(c.y) * res.z + u32(c.z)) * res.x + u32(c.x);
    let w = S[S[b + 12u] + (idx >> 1u)];
    return f32((w >> ((idx & 1u) * 16u)) & 0xffffu);
}

// σt nel punto (interpolazione trilineare, 0 fuori dalla griglia).
fn fog_density(b: u32, p: vec3<f32>) -> f32 {
    let res = fog_res(b);
    let cell = sf(b + 3u);
    var q = (p - sv3(b)) / cell;
    let size = vec3<f32>(res);
    if (!(q.x >= 0.0 && q.y >= 0.0 && q.z >= 0.0 && q.x <= size.x && q.y <= size.y && q.z <= size.z)) { return 0.0; }
    q = q - vec3<f32>(0.5);
    let f = floor(q);
    let t = q - f;
    let i = vec3<i32>(f);
    let x00 = mix(fog_texel(b, res, i), fog_texel(b, res, i + vec3<i32>(1, 0, 0)), t.x);
    let x10 = mix(fog_texel(b, res, i + vec3<i32>(0, 1, 0)), fog_texel(b, res, i + vec3<i32>(1, 1, 0)), t.x);
    let x01 = mix(fog_texel(b, res, i + vec3<i32>(0, 0, 1)), fog_texel(b, res, i + vec3<i32>(1, 0, 1)), t.x);
    let x11 = mix(fog_texel(b, res, i + vec3<i32>(0, 1, 1)), fog_texel(b, res, i + vec3<i32>(1, 1, 1)), t.x);
    return mix(mix(x00, x10, t.y), mix(x01, x11, t.y), t.z) * sf(b + 7u);
}

// Intervallo del raggio nel riquadro della nebbia, limitato a [0, tmax]; x > y se vuoto.
fn fog_clip(b: u32, o: vec3<f32>, d: vec3<f32>, tmax: f32) -> vec2<f32> {
    let lo = sv3(b);
    let hi = lo + vec3<f32>(fog_res(b)) * sf(b + 3u);
    let inv = 1.0 / d;
    let t0 = (lo - o) * inv;
    let t1 = (hi - o) * inv;
    let tn = max(max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z)), 0.0);
    let tf = min(min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z)), tmax);
    return vec2<f32>(tn, tf);
}

// Tratto a majorant costante da t (al più fino a t1): (fine, majorant). Griglia 3D di blocchi
// percorsa con un DDA, come BakedFog::majorant_segment sulla CPU.
fn fog_segment(b: u32, o: vec3<f32>, d: vec3<f32>, t: f32, t1: f32) -> vec2<f32> {
    let res = fog_res(b);
    let bs = S[b + 14u];
    let side = sf(b + 3u) * f32(bs);
    let mres = (res + vec3<u32>(bs - 1u)) / bs;
    let mn = sv3(b);
    // Blocco in cui il raggio sta entrando (punto un filo più avanti).
    let q = (o + d * (t + 1e-4 * side) - mn) / side;
    let blk = vec3<u32>(min(max(floor(q), vec3<f32>(0.0)), vec3<f32>(mres - vec3<u32>(1u))));
    var tb = t1;
    for (var a: u32 = 0u; a < 3u; a = a + 1u) {
        if (d[a] > 1e-9) {
            tb = min(tb, (mn[a] + f32(blk[a] + 1u) * side - o[a]) / d[a]);
        } else if (d[a] < -1e-9) {
            tb = min(tb, (mn[a] + f32(blk[a]) * side - o[a]) / d[a]);
        }
    }
    tb = min(max(tb, t + 1e-6 * (1.0 + abs(t))), t1);
    let idx = (blk.y * mres.z + blk.z) * mres.x + blk.x;
    return vec2<f32>(tb, sf(S[b + 13u] + idx));
}

// Prima collisione reale (delta tracking): (t, indice nebbia), t < 0 se nessuna.
fn fog_collision(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> vec2<f32> {
    var best_t = -1.0;
    var best_k = 0u;
    for (var k: u32 = 0u; k < pu(P_N_FOGS); k = k + 1u) {
        let b = fog_base(k);
        var limit = tmax;
        if (best_t >= 0.0) { limit = best_t; }
        let c = fog_clip(b, o, d, limit);
        var t = c.x;
        var found = false;
        var guard = 0u;
        loop {
            if (t >= c.y || found || guard > 100000u) { break; }
            guard = guard + 1u;
            let seg = fog_segment(b, o, d, t, c.y);
            if (seg.y <= 0.0) { t = seg.x; continue; }
            var ts = t;
            loop {
                ts = ts - log(1.0 - rnd()) / seg.y;
                if (ts >= seg.x) { break; }
                if (rnd() * seg.y < fog_density(b, o + d * ts)) { found = true; break; }
            }
            if (found) { best_t = ts; best_k = k; } else { t = seg.x; }
        }
    }
    return vec2<f32>(best_t, f32(best_k));
}

// Trasmittanza stimata con il ratio tracking.
fn fog_transmittance(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> f32 {
    var tr = 1.0;
    for (var k: u32 = 0u; k < pu(P_N_FOGS); k = k + 1u) {
        let b = fog_base(k);
        let c = fog_clip(b, o, d, tmax);
        var t = c.x;
        var guard = 0u;
        loop {
            if (t >= c.y || tr == 0.0 || guard > 100000u) { break; }
            guard = guard + 1u;
            let seg = fog_segment(b, o, d, t, c.y);
            if (seg.y > 0.0) {
                var ts = t;
                loop {
                    ts = ts - log(1.0 - rnd()) / seg.y;
                    if (ts >= seg.x) { break; }
                    tr = tr * (1.0 - min(fog_density(b, o + d * ts) / seg.y, 1.0));
                    if (tr < 0.1) {
                        if (rnd() < 0.5) { tr = 0.0; break; }
                        tr = tr * 2.0;
                    }
                }
            }
            t = seg.x;
        }
        if (tr == 0.0) { return 0.0; }
    }
    return tr;
}

fn hg_pdf(g: f32, c: f32) -> f32 {
    let den = 1.0 + g * g - 2.0 * g * c;
    return (1.0 - g * g) / (4.0 * PI * den * sqrt(max(den, 1e-12)));
}

fn hg_sample(g: f32, d: vec3<f32>, u1: f32, u2: f32) -> vec3<f32> {
    var c = 1.0 - 2.0 * u1;
    if (abs(g) >= 1e-3) {
        let s = (1.0 - g * g) / (1.0 + g - 2.0 * g * u1);
        c = clamp((1.0 + g * g - s * s) / (2.0 * g), -1.0, 1.0);
    }
    let sn = sqrt(max(1.0 - c * c, 0.0));
    let phi = 2.0 * PI * u2;
    return to_world(onb(d), vec3<f32>(sn * cos(phi), sn * sin(phi), c));
}

// Visibilità di una luce campionata da `so`: 0 se coperta, altrimenti trasmittanza delle nebbie.
fn light_visibility(so: vec3<f32>, ls: LSample) -> f32 {
    if (!ls.shadows) { return 1.0; }
    var tmax = INF;
    if (ls.dist < INF) { tmax = ls.dist * (1.0 - 1e-4); }
    if (trace(so, ls.wi, 0.0, tmax, true).found) { return 0.0; }
    if (pu(P_N_FOGS) == 0u) { return 1.0; }
    return fog_transmittance(so, ls.wi, tmax);
}

// ---------------------------------------------------------------- Volumi trasparenti
// Volumi in cui si trova il percorso (vetro, liquido), al più 4 annidati: il corrente è
// quello con priorità più alta (a pari priorità l'ultimo in cui si è entrati). Stesso
// algoritmo di `Media` nell'integratore CPU.
var<private> media_ids: array<u32, 4>;
var<private> media_n: u32;

fn mat_word(i: u32, k: u32) -> f32 { return sf(pu(P_OFF_MATS) + i * MAT_WORDS + k); }

fn media_current() -> i32 {
    var best = -1;
    for (var k: u32 = 0u; k < media_n; k = k + 1u) {
        let m = media_ids[k];
        if (best < 0 || mat_word(m, 33u) >= mat_word(u32(best), 33u)) { best = i32(m); }
    }
    return best;
}

fn media_contains(m: u32) -> bool {
    for (var k: u32 = 0u; k < media_n; k = k + 1u) {
        if (media_ids[k] == m) { return true; }
    }
    return false;
}

fn media_false_hit(m: u32, front: bool) -> bool {
    if (mat_word(m, 8u) <= 0.0) { return false; }
    let cur = media_current();
    if (cur < 0) { return false; }
    let p = mat_word(m, 33u);
    let pc = mat_word(u32(cur), 33u);
    if (front) { return p < pc; }
    return u32(cur) != m && media_contains(m) && p < pc;
}

fn media_cross(m: u32, front: bool) {
    if (front) {
        if (media_n < 4u) { media_ids[media_n] = m; media_n = media_n + 1u; }
        return;
    }
    var found = -1;
    for (var k: u32 = 0u; k < media_n; k = k + 1u) {
        if (media_ids[k] == m) { found = i32(k); }
    }
    if (found < 0) { return; }
    for (var k: u32 = u32(found); k + 1u < media_n; k = k + 1u) { media_ids[k] = media_ids[k + 1u]; }
    media_n = media_n - 1u;
}

// IOR dall'altra parte della superficie del materiale m (aria o il volume attorno).
fn media_outside_ior(m: u32, front: bool) -> f32 {
    let saved_ids = media_ids;
    let saved_n = media_n;
    if (!front) { media_cross(m, false); }
    let cur = media_current();
    media_ids = saved_ids;
    media_n = saved_n;
    if (cur < 0) { return 1.0; }
    return mat_word(u32(cur), 5u);
}

fn media_transmittance(t: f32) -> vec3<f32> {
    let cur = media_current();
    if (cur < 0) { return vec3<f32>(1.0); }
    let b = pu(P_OFF_MATS) + u32(cur) * MAT_WORDS + 30u;
    return exp(-sv3(b) * t);
}

// ---------------------------------------------------------------- Integratore
fn radiance(o_in: vec3<f32>, d_in: vec3<f32>) -> vec3<f32> {
    media_n = 0u;
    var o = o_in;
    var d = d_in;
    var l = vec3<f32>(0.0);
    var beta = vec3<f32>(1.0);
    var specular = true;
    var prev_pdf = 0.0;
    let n_lights = pu(P_N_LIGHTS);
    let n_select = pu(P_N_SELECT);
    let has_env_map = pu(P_ENV_KIND) == 2u;
    let has_em = pu(P_N_EMISSIVE) > 0u;
    var sel_pdf = 0.0;
    if (n_select > 0u) { sel_pdf = 1.0 / f32(n_select); }
    let has_env = has_env_map || pu(P_N_PORTALS) > 0u;
    var env_slot = 0xffffffffu;
    if (has_env) { env_slot = n_lights; }
    let clamp_v = pf(P_CLAMP);
    let max_b = pu(P_MAX_BOUNCES);

    for (var depth: u32 = 0u; depth < max_b; depth = depth + 1u) {
        var contrib_scale = 1.0;
        var hit = trace(o, d, 0.0, INF, false);
        // Facce di volumi con priorità più bassa del corrente: si attraversano senza rimbalzo.
        for (var skip: u32 = 0u; skip < 8u && hit.found; skip = skip + 1u) {
            let hpb = pu(P_OFF_PRIMS) + hit.prim * PRIM_WORDS;
            let hm = S[hpb + 3u];
            let hng = sv3(hpb + 12u);
            let hfront = dot(hng, d) < 0.0;
            if (!media_false_hit(hm, hfront)) { break; }
            beta = beta * media_transmittance(hit.t);
            media_cross(hm, hfront);
            var hside = hng;
            if (hfront) { hside = -hng; }
            o = offset_ray(o + d * hit.t, hside);
            hit = trace(o, d, 0.0, INF, false);
        }
        var t_surf = INF;
        if (hit.found) { t_surf = hit.t; }

        // Luci ad area visibili.
        var lt = -1.0;
        var lpdf = 0.0;
        var lfront = 0.0;
        var lem = vec3<f32>(0.0);
        for (var i: u32 = 0u; i < n_lights; i = i + 1u) {
            if (depth == 0u && S[light_base(i) + 2u] == 0u) { continue; }
            var tm = t_surf;
            if (lt > 0.0) { tm = lt; }
            let r = intersect_area(i, o, d, tm);
            if (r.x > 0.0) { lt = r.x; lpdf = r.y; lfront = r.z; lem = sv3(light_base(i) + 4u); }
        }
        // Assorbimento del volume trasparente attraversato.
        if (media_n > 0u) {
            var t_end = t_surf;
            if (lt > 0.0) { t_end = lt; }
            if (t_end < INF) { beta = beta * media_transmittance(t_end); }
        }
        // Nebbie: collisione prima della superficie o della luce (delta tracking).
        if (pu(P_N_FOGS) > 0u) {
            var t_end = t_surf;
            if (lt > 0.0) { t_end = lt; }
            let ev = fog_collision(o, d, t_end);
            if (ev.x >= 0.0) {
                let fb = fog_base(u32(ev.y));
                let p = o + d * ev.x;
                beta = beta * sv3(fb + 8u);
                if (beta.x == 0.0 && beta.y == 0.0 && beta.z == 0.0) { break; }
                let g = sf(fb + 11u);
                if (n_select > 0u) {
                    let li_idx = min(u32(rnd() * f32(n_select)), n_select - 1u);
                    let u1 = rnd();
                    let u2 = rnd();
                    var ls: LSample;
                    if (li_idx < n_lights) {
                        ls = sample_light(li_idx, p, u1, u2);
                    } else if (li_idx == env_slot) {
                        ls = env_light_sample(p, u1, u2);
                    } else {
                        ls = emissive_sample(p, rnd(), u1, u2);
                    }
                    if (ls.ok) {
                        let ph = hg_pdf(g, dot(d, ls.wi));
                        let vis = light_visibility(p, ls);
                        if (vis > 0.0) {
                            var c: vec3<f32>;
                            if (ls.delta) {
                                c = ls.li * (ph / sel_pdf);
                            } else {
                                let lp = ls.pdf * sel_pdf;
                                c = ls.li * (ph * power_heuristic(lp, ph) / lp);
                            }
                            c = beta * c * vis;
                            if (clamp_v > 0.0 && depth > 0u && luminance(c) > clamp_v) { c = c * (clamp_v / luminance(c)); }
                            l = l + c;
                        }
                    }
                }
                let wi = hg_sample(g, d, rnd(), rnd());
                prev_pdf = hg_pdf(g, dot(d, wi));
                specular = false;
                o = p;
                d = wi;
                if (depth + 1u >= pu(P_RR_START)) {
                    let q = min(max(max(beta.x, beta.y), beta.z), 0.95);
                    if (rnd() >= q) { break; }
                    beta = beta / q;
                }
                continue;
            }
        }

        if (lt > 0.0) {
            if (lfront > 0.0) {
                var w = 1.0;
                if (!specular) { w = power_heuristic(prev_pdf, lpdf * sel_pdf); }
                var c = beta * lem * w;
                if (clamp_v > 0.0 && depth > 0u && luminance(c) > clamp_v) { c = c * (clamp_v / luminance(c)); }
                l = l + c;
            }
            break;
        }

        if (!hit.found) {
            var c = vec3<f32>(0.0);
            if (depth == 0u) {
                if ((pu(P_FLAGS) & 1u) != 0u) { c = beta * env_radiance(d, true); }
            } else if (has_env) {
                var w = 1.0;
                if (!specular) { w = power_heuristic(prev_pdf, env_light_pdf(o, d) * sel_pdf); }
                c = beta * env_radiance(d, false) * w;
            } else {
                c = beta * env_radiance(d, false);
            }
            if (clamp_v > 0.0 && depth > 0u && luminance(c) > clamp_v) { c = c * (clamp_v / luminance(c)); }
            l = l + c;
            break;
        }

        let prim = hit.prim;
        let pb = pu(P_OFF_PRIMS) + prim * PRIM_WORDS;
        let p = o + d * hit.t;
        let mat_id = S[pb + 3u];
        var m = load_mat(mat_id);
        var ng = sv3(pb + 12u);
        let front_face = dot(ng, d) < 0.0;
        if (m.transmission > 0.0) {
            // IOR relativo al volume dall'altra parte (aria, o il vetro attorno al liquido).
            m.ior = max(m.ior / media_outside_ior(mat_id, front_face), 1e-3);
            m.f0d = ((m.ior - 1.0) / (m.ior + 1.0)) * ((m.ior - 1.0) / (m.ior + 1.0));
        }
        var ns = sv3(pb) * (1.0 - hit.u - hit.v) + sv3(pb + 4u) * hit.u + sv3(pb + 8u) * hit.v;
        if (dot(ns, ns) > 0.0) { ns = normalize(ns); } else { ns = ng; }
        if (has_textures(m)) { apply_textures(&m, prim, hit.u, hit.v, p, &ns); }
        let wo = -d;
        let entering = dot(ng, wo) > 0.0;
        if (!entering) { ng = -ng; ns = -ns; }
        if (dot(ns, wo) <= 0.0 || dot(ns, ng) <= 0.0) { ns = ng; }

        if (dot(m.emission, m.emission) > 0.0) {
            var w = 1.0;
            if (has_em && !specular) {
                let cos_l = max(abs(dot(ng, wo)), 1e-8);
                w = power_heuristic(prev_pdf, emissive_area_pdf(prim) * hit.t * hit.t / cos_l * sel_pdf);
            }
            var c = beta * m.emission * w;
            if (clamp_v > 0.0 && depth > 0u && luminance(c) > clamp_v) { c = c * (clamp_v / luminance(c)); }
            l = l + c;
        }

        var bs: Bsdf;
        bs.frame = onb(ns);
        bs.ng = ng;
        bs.m = m;
        bs.entering = entering;

        // Next Event Estimation.
        if (n_select > 0u && has_non_delta(bs)) {
            let li_idx = min(u32(rnd() * f32(n_select)), n_select - 1u);
            let u1 = rnd();
            let u2 = rnd();
            var ls: LSample;
            if (li_idx < n_lights) {
                ls = sample_light(li_idx, p, u1, u2);
            } else if (li_idx == env_slot) {
                ls = env_light_sample(p, u1, u2);
            } else {
                ls = emissive_sample(p, rnd(), u1, u2);
            }
            if (ls.ok) {
                let e = bsdf_eval(bs, wo, ls.wi);
                if (e.x != 0.0 || e.y != 0.0 || e.z != 0.0) {
                    let cosv = abs(dot(ls.wi, ns));
                    var side = ng;
                    if (dot(ls.wi, ng) < 0.0) { side = -ng; }
                    let vis = light_visibility(offset_ray(p, side), ls);
                    if (vis > 0.0) {
                        var c: vec3<f32>;
                        if (ls.delta) {
                            c = e.xyz * ls.li * (cosv / sel_pdf);
                        } else {
                            let lp = ls.pdf * sel_pdf;
                            c = e.xyz * ls.li * (cosv * power_heuristic(lp, e.w) / lp);
                        }
                        c = beta * c * vis;
                        if (clamp_v > 0.0 && depth > 0u && luminance(c) > clamp_v) { c = c * (clamp_v / luminance(c)); }
                        l = l + c;
                    }
                }
            }
        }

        // Campionamento della BSDF.
        let ul = rnd();
        let u1 = rnd();
        let u2 = rnd();
        let s = bsdf_sample(bs, wo, ul, u1, u2);
        if (!s.ok) { break; }
        beta = beta * s.weight;
        if (!(beta.x < INF && beta.y < INF && beta.z < INF) || (beta.x == 0.0 && beta.y == 0.0 && beta.z == 0.0)) { break; }
        specular = s.delta;
        prev_pdf = s.pdf;
        if (dot(s.wi, ng) < 0.0 && m.transmission > 0.0) { media_cross(mat_id, front_face); }
        var side = ng;
        if (dot(s.wi, ng) < 0.0) { side = -ng; }
        o = offset_ray(p, side);
        d = s.wi;

        if (depth + 1u >= pu(P_RR_START)) {
            let q = min(max(max(beta.x, beta.y), beta.z), 0.95);
            if (rnd() >= q) { break; }
            beta = beta / q;
        }
    }
    return l;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w = pu(P_WIDTH);
    let h = pu(P_HEIGHT);
    if (gid.x >= w || gid.y >= h) { return; }
    let idx = gid.y * w + gid.x;
    if ((pu(P_FLAGS) & 2u) != 0u) {
        // Riquadro già abbastanza pulito: nessun campione (la sessione ignora il pixel).
        let tile = (gid.y / ADAPTIVE_TILE) * pu(P_TILES_X) + gid.x / ADAPTIVE_TILE;
        if (((MASK[tile >> 5u] >> (tile & 31u)) & 1u) == 0u) {
            OUT[idx] = vec4<f32>(0.0);
            return;
        }
    }
    let first = pu(P_FIRST_SAMPLE);
    let n = pu(P_N_SAMPLES);
    let cam_pos = pv3(P_CAM_POS);
    let r = pv3(P_CAM_R);
    let u = pv3(P_CAM_U);
    let f = pv3(P_CAM_F);
    let th = pf(P_TAN_HALF);
    let aspect = pf(P_ASPECT);
    var sum = vec3<f32>(0.0);
    for (var s: u32 = 0u; s < n; s = s + 1u) {
        rng = pcg_hash(gid.x ^ pcg_hash(gid.y ^ pcg_hash((first + s) ^ pcg_hash(pu(P_SEED_LO) ^ pcg_hash(pu(P_SEED_HI))))));
        let jx = rnd();
        let jy = rnd();
        let ndc_x = 2.0 * (f32(gid.x) + jx) / f32(w) - 1.0 + 2.0 * pf(P_SHIFT_X);
        let ndc_y = 1.0 - 2.0 * (f32(gid.y) + jy) / f32(h) + 2.0 * pf(P_SHIFT_Y);
        var dir = normalize(f + r * (ndc_x * th * aspect) + u * (ndc_y * th));
        var origin = cam_pos;
        let lr = pf(P_LENS_R);
        if (lr > 0.0) {
            // Lente sottile messa a fuoco sul piano a distanza P_FOCUS.
            let focus = cam_pos + dir * (pf(P_FOCUS) / max(dot(dir, f), 1e-6));
            let u0 = rnd();
            let u1 = rnd();
            let u2 = rnd();
            let a = sample_aperture(u0, u1, u2) * lr;
            origin = cam_pos + r * a.x + u * a.y;
            dir = normalize(focus - origin);
        }
        let c = radiance(origin, dir);
        // Campioni non finiti scartati (come nell'accumulatore CPU).
        if (c.x < INF && c.y < INF && c.z < INF && c.x == c.x && c.y == c.y && c.z == c.z) { sum = sum + c; }
    }
    OUT[idx] = vec4<f32>(sum, 0.0);
}
