// ================================================================ Render spettrale
// Si compila in coda a path_tracer.wgsl (ne usa tracciamento, luci, texture, BSDF scalari).
// Stesso integratore con la radianza a quattro lunghezze d'onda (hero wavelength), vedi
// `forge_render_core::spectral` e `forge_render_cpu::spectral`. I dati (matrice XYZ → RGB, ∫ȳ,
// tabella dei coefficienti RGB → spettro) stanno in SPEC, caricato solo in modalità spettrale.

@group(0) @binding(4) var<storage, read> SPEC: array<f32>;

const SP_RES: u32 = 64u;
const SP_Y_INT: u32 = 9u;
const SP_Z: u32 = 10u;
const SP_C: u32 = 74u;
const LAMBDA_MIN: f32 = 360.0;
const LAMBDA_MAX: f32 = 830.0;
const VIS_A: f32 = 0.0072;
const VIS_B: f32 = 538.0;

var<private> lam: vec4<f32>;
var<private> lam_pdf: vec4<f32>;

fn visible_pdf(l: f32) -> f32 {
    if (l < LAMBDA_MIN || l > LAMBDA_MAX) { return 0.0; }
    let t0 = tanh(VIS_A * (LAMBDA_MIN - VIS_B));
    let t1 = tanh(VIS_A * (LAMBDA_MAX - VIS_B));
    let c = cosh(VIS_A * (l - VIS_B));
    return VIS_A / (t1 - t0) / (c * c);
}

fn sample_visible(u: f32) -> f32 {
    let t0 = tanh(VIS_A * (LAMBDA_MIN - VIS_B));
    let t1 = tanh(VIS_A * (LAMBDA_MAX - VIS_B));
    let t = clamp(t0 + u * (t1 - t0), -0.9999999, 0.9999999);
    return clamp(VIS_B + atanh(t) / VIS_A, LAMBDA_MIN, LAMBDA_MAX);
}

fn sample_wavelengths(u: f32) {
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        var up = u + f32(i) * 0.25;
        if (up >= 1.0) { up = up - 1.0; }
        lam[i] = sample_visible(up);
        lam_pdf[i] = visible_pdf(lam[i]);
    }
}

fn secondary_terminated() -> bool { return lam_pdf.y == 0.0 && lam_pdf.z == 0.0 && lam_pdf.w == 0.0; }

fn sp_gauss(l: f32, mu: f32, s1: f32, s2: f32) -> f32 {
    var s = s2;
    if (l < mu) { s = s1; }
    let t = (l - mu) / s;
    return exp(-0.5 * t * t);
}

// Funzioni colorimetriche CIE 1931 (Wyman, Sloan e Shirley 2013).
fn cie_xyz(l: f32) -> vec3<f32> {
    let x = 1.056 * sp_gauss(l, 599.8, 37.9, 31.0) + 0.362 * sp_gauss(l, 442.0, 16.0, 26.7) - 0.065 * sp_gauss(l, 501.1, 20.4, 26.2);
    let y = 0.821 * sp_gauss(l, 568.8, 46.9, 40.5) + 0.286 * sp_gauss(l, 530.9, 16.3, 31.1);
    let z = 1.217 * sp_gauss(l, 437.0, 11.8, 36.0) + 0.681 * sp_gauss(l, 459.0, 26.0, 13.8);
    return vec3<f32>(x, y, z);
}

fn spectral_to_rgb(L: vec4<f32>) -> vec3<f32> {
    var xyz = vec3<f32>(0.0);
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        if (lam_pdf[i] > 0.0 && L[i] != 0.0) { xyz = xyz + cie_xyz(lam[i]) * (L[i] / lam_pdf[i]); }
    }
    xyz = xyz / (4.0 * SPEC[SP_Y_INT]);
    return vec3<f32>(
        SPEC[0] * xyz.x + SPEC[1] * xyz.y + SPEC[2] * xyz.z,
        SPEC[3] * xyz.x + SPEC[4] * xyz.y + SPEC[5] * xyz.z,
        SPEC[6] * xyz.x + SPEC[7] * xyz.y + SPEC[8] * xyz.z,
    );
}

fn sp_coef(m: u32, z: u32, y: u32, x: u32) -> vec3<f32> {
    let i = SP_C + ((((m * SP_RES + z) * SP_RES + y) * SP_RES + x) * 3u);
    return vec3<f32>(SPEC[i], SPEC[i + 1u], SPEC[i + 2u]);
}

// Coefficienti della sigmoide per un RGB in [0, 1] (interpolazione trilineare).
fn rgb_coeffs(rgb_in: vec3<f32>) -> vec3<f32> {
    let rgb = clamp(rgb_in, vec3<f32>(0.0), vec3<f32>(1.0));
    if (rgb.x == rgb.y && rgb.y == rgb.z) {
        let v = clamp(rgb.x, 1e-6, 1.0 - 1e-6);
        return vec3<f32>(0.0, 0.0, (v - 0.5) / sqrt(v * (1.0 - v)));
    }
    var m = 2u;
    if (rgb.x >= rgb.y && rgb.x >= rgb.z) { m = 0u; } else if (rgb.y >= rgb.z) { m = 1u; }
    let z = rgb[m];
    let fx = rgb[(m + 1u) % 3u] / z * f32(SP_RES - 1u);
    let fy = rgb[(m + 2u) % 3u] / z * f32(SP_RES - 1u);
    // Nodo z: ricerca binaria.
    var lo = 0u;
    var hi = SP_RES - 1u;
    while (hi - lo > 1u) {
        let mid = (lo + hi) / 2u;
        if (SPEC[SP_Z + mid] <= z) { lo = mid; } else { hi = mid; }
    }
    let zi = min(lo, SP_RES - 2u);
    let xi = min(u32(fx), SP_RES - 2u);
    let yi = min(u32(fy), SP_RES - 2u);
    let dx = fx - f32(xi);
    let dy = fy - f32(yi);
    let dz = clamp((z - SPEC[SP_Z + zi]) / (SPEC[SP_Z + zi + 1u] - SPEC[SP_Z + zi]), 0.0, 1.0);
    let c00 = mix(sp_coef(m, zi, yi, xi), sp_coef(m, zi, yi, xi + 1u), dx);
    let c01 = mix(sp_coef(m, zi, yi + 1u, xi), sp_coef(m, zi, yi + 1u, xi + 1u), dx);
    let c10 = mix(sp_coef(m, zi + 1u, yi, xi), sp_coef(m, zi + 1u, yi, xi + 1u), dx);
    let c11 = mix(sp_coef(m, zi + 1u, yi + 1u, xi), sp_coef(m, zi + 1u, yi + 1u, xi + 1u), dx);
    return mix(mix(c00, c01, dy), mix(c10, c11, dy), dz);
}

fn sp_sigmoid(x: f32) -> f32 {
    if (abs(x) > 1e18) { return select(0.0, 1.0, x > 0.0); }
    return 0.5 + x / (2.0 * sqrt(1.0 + x * x));
}

fn sp_eval4(c: vec3<f32>) -> vec4<f32> {
    var r: vec4<f32>;
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        let t = (lam[i] - LAMBDA_MIN) / (LAMBDA_MAX - LAMBDA_MIN);
        r[i] = sp_sigmoid((c.x * t + c.y) * t + c.z);
    }
    return r;
}

// Riflettanza RGB → valori alle quattro lunghezze d'onda.
fn albedo4(rgb: vec3<f32>) -> vec4<f32> {
    if (rgb.x == 0.0 && rgb.y == 0.0 && rgb.z == 0.0) { return vec4<f32>(0.0); }
    return sp_eval4(rgb_coeffs(rgb));
}

// Radianza RGB (senza limite) → valori alle quattro lunghezze d'onda: forma dalla tabella,
// intensità nella scala 2·max.
fn illum4(rgb_in: vec3<f32>) -> vec4<f32> {
    let rgb = max(rgb_in, vec3<f32>(0.0));
    let mx = max(max(rgb.x, rgb.y), rgb.z);
    if (!(mx > 0.0) || !(mx < INF)) { return vec4<f32>(0.0); }
    let sc = 2.0 * mx;
    return sp_eval4(rgb_coeffs(rgb / sc)) * sc;
}

fn cauchy_ior(nd: f32, abbe: f32, l: f32) -> f32 {
    if (abbe <= 0.0 || nd <= 1.0) { return nd; }
    let lf = 0.48613;
    let lc = 0.65627;
    let ld = 0.58756;
    let b = (nd - 1.0) / (abbe * (1.0 / (lf * lf) - 1.0 / (lc * lc)));
    let a = nd - b / (ld * ld);
    let lu = l * 1e-3;
    return a + b / (lu * lu);
}

fn mat_ior_hero(m: u32) -> f32 { return cauchy_ior(mat_word(m, 5u), mat_word(m, 34u), lam.x); }

// Volume dall'altra parte della superficie del materiale m (-1 = aria).
fn media_outside_id(m: u32, front: bool) -> i32 {
    let saved_ids = media_ids;
    let saved_n = media_n;
    if (!front) { media_cross(m, false); }
    let cur = media_current();
    media_ids = saved_ids;
    media_n = saved_n;
    return cur;
}

fn media_transmittance4(t: f32) -> vec4<f32> {
    let cur = media_current();
    if (cur < 0) { return vec4<f32>(1.0); }
    let b = pu(P_OFF_MATS) + u32(cur) * MAT_WORDS;
    let sigma = sv3(b + 30u);
    let depth = sf(b + 35u);
    if (depth <= 0.0 || (sigma.x == 0.0 && sigma.y == 0.0 && sigma.z == 0.0)) { return vec4<f32>(1.0); }
    // Colore del volume allo spessore di riferimento, poi Beer-Lambert per lunghezza d'onda.
    let r = max(albedo4(exp(-sigma * depth)), vec4<f32>(1e-6));
    return pow(r, vec4<f32>(t / depth));
}

fn schlick4(f0: vec4<f32>, c: f32) -> vec4<f32> {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    return f0 + (vec4<f32>(1.0) - f0) * (m2 * m2 * m);
}

struct SEval { f: vec4<f32>, pdf: f32 };

fn eval_opaque_s(bs: Bsdf, col: vec4<f32>, wo: vec3<f32>, wi: vec3<f32>) -> SEval {
    var r: SEval;
    r.f = vec4<f32>(0.0);
    r.pdf = 0.0;
    if (wo.z <= 0.0 || wi.z <= 0.0) { return r; }
    let m = bs.m;
    let ps = spec_prob(bs, wo);
    let a2 = m.alpha * m.alpha;
    let h = normalize(wo + wi);
    if (ps > 0.0) {
        let d = ggx_d(h, a2);
        let lo = smith_lambda(wo, a2);
        let li = smith_lambda(wi, a2);
        let g2 = 1.0 / (1.0 + lo + li);
        let g1o = 1.0 / (1.0 + lo);
        let voh = max(dot(wo, h), 0.0);
        let fr = mix(vec4<f32>(f_diel(bs, voh)), schlick4(col, voh), m.metallic);
        let e = ggx_albedo(wo.z, m.alpha);
        let f0 = mix(vec4<f32>(min(m.spec_scale * m.f0d, 1.0)), col, m.metallic);
        let ms = vec4<f32>(1.0) + f0 * (1.0 / e - 1.0);
        r.f = r.f + fr * ms * (d * g2 / (4.0 * wo.z * wi.z));
        r.pdf = r.pdf + ps * g1o * d / (4.0 * wo.z);
    }
    if (ps < 1.0) {
        let kd = (1.0 - m.metallic) * (1.0 - f_diel(bs, wi.z)) * (1.0 - f_diel(bs, wo.z));
        r.f = r.f + col * (kd * INV_PI);
        r.pdf = r.pdf + (1.0 - ps) * max(wi.z, 0.0) * INV_PI;
    }
    return r;
}

fn bsdf_eval_s(bs: Bsdf, col: vec4<f32>, wo_w: vec3<f32>, wi_w: vec3<f32>) -> SEval {
    var r: SEval;
    r.f = vec4<f32>(0.0);
    r.pdf = 0.0;
    if (dot(wo_w, bs.ng) <= 0.0) { return r; }
    let wo = to_local(bs.frame, wo_w);
    let wi = to_local(bs.frame, wi_w);
    let geo_refl = dot(wi_w, bs.ng) > 0.0;
    if (geo_refl != (wi.z > 0.0)) { return r; }
    let t = bs.m.transmission;
    if (t < 1.0 && geo_refl) {
        let o = eval_opaque_s(bs, col, wo, wi);
        r.f = r.f + o.f * (1.0 - t);
        r.pdf = r.pdf + o.pdf * (1.0 - t);
    }
    if (rough_glass(bs)) {
        // Vetro satinato con colore bianco: la parte scalare; il colore tinge solo la trasmissione.
        var white = bs;
        white.m.base = vec3<f32>(1.0);
        let g = eval_rough_glass(white, wo, wi);
        if (wi.z > 0.0) { r.f = r.f + vec4<f32>(g.x) * t; } else { r.f = r.f + col * (g.x * t); }
        r.pdf = r.pdf + g.w * t;
    }
    return r;
}

struct BSampleS { wi: vec3<f32>, weight: vec4<f32>, pdf: f32, delta: bool, ok: bool };

fn bsdf_sample_s(bs: Bsdf, col: vec4<f32>, wo_w: vec3<f32>, u_lobe: f32, u1: f32, u2: f32) -> BSampleS {
    var s: BSampleS;
    s.ok = false;
    s.delta = false;
    let m = bs.m;
    let wo = to_local(bs.frame, wo_w);
    if (wo.z <= 0.0) { return s; }
    var wi: vec3<f32>;
    if (u_lobe < m.transmission) {
        let ip = ior_pair(bs);
        if (!rough_glass(bs)) {
            let n = bs.frame.n;
            let cos_o = dot(wo_w, n);
            if (cos_o <= 0.0) { return s; }
            let fc = fresnel_dielectric(cos_o, ip.y / ip.x);
            s.delta = true;
            s.pdf = 0.0;
            s.ok = true;
            if (u1 < fc.x) {
                s.wi = 2.0 * cos_o * n - wo_w;
                s.weight = vec4<f32>(1.0);
                return s;
            }
            let eta = ip.x / ip.y;
            s.wi = normalize(-wo_w * eta + n * (eta * cos_o - fc.y));
            s.weight = col * (eta * eta);
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
    let e = bsdf_eval_s(bs, col, wo_w, wi_w);
    if (e.pdf <= 0.0 || all(e.f == vec4<f32>(0.0))) { return s; }
    s.wi = wi_w;
    s.weight = e.f * (abs(dot(wi_w, bs.frame.n)) / e.pdf);
    s.pdf = e.pdf;
    s.ok = true;
    return s;
}

// I due percorsi della tabella attorno alla lunghezza d'onda l: (indice del primo, peso del secondo).
fn laser_lambda_pair(l: f32) -> vec2<f32> {
    let k = clamp((l - 380.0) / 400.0 * f32(LASER_N_LAMBDA - 1u), 0.0, f32(LASER_N_LAMBDA - 1u));
    let k0 = min(floor(k), f32(LASER_N_LAMBDA - 2u));
    return vec2<f32>(k0, k - k0);
}

// Stessa struttura (stesse superfici attraversate): si può interpolare tra i due percorsi.
fn laser_compatible(pa: u32, pb: u32) -> bool {
    if (S[pa] != S[pb] || S[pa + 1u] != S[pb + 1u]) { return false; }
    for (var s: u32 = 0u; s < S[pa]; s = s + 1u) {
        if (S[pa + 16u + s * 12u + 3u] != S[pb + 16u + s * 12u + 3u]) { return false; }
    }
    return true;
}

// Fascio visibile dei laser: per ogni lunghezza d'onda il suo percorso (dispersione nei vetri),
// interpolato tra le due lunghezze d'onda della tabella più vicine.
fn laser_beams_s(o: vec3<f32>, d: vec3<f32>, t_end: f32) -> vec4<f32> {
    var c = vec4<f32>(0.0);
    for (var li: u32 = 0u; li < pu(P_N_LASERS); li = li + 1u) {
        let lb = laser_base(li);
        let dens = sf(lb + 4u);
        if (dens <= 0.0) { continue; }
        let r = sf(lb + 3u);
        let power = illum4(sv3(lb));
        for (var i: u32 = 0u; i < 4u; i = i + 1u) {
            if (lam_pdf[i] <= 0.0) { continue; }
            let kp = laser_lambda_pair(lam[i]);
            let p0 = laser_path(li, 1u + u32(kp.x));
            let p1 = laser_path(li, 2u + u32(kp.x));
            let lerp = laser_compatible(p0, p1);
            var pb = p0;
            if (!lerp && kp.y >= 0.5) { pb = p1; }
            for (var s: u32 = 0u; s < S[pb]; s = s + 1u) {
                let sb = pb + 16u + s * 12u;
                if (S[sb + 3u] != 0u) { continue; }
                var a = sv3(sb);
                var e = sv3(sb + 4u);
                var tp = sf(sb + 8u);
                if (lerp) {
                    let sb1 = p1 + 16u + s * 12u;
                    a = mix(a, sv3(sb1), kp.y);
                    e = mix(e, sv3(sb1 + 4u), kp.y);
                    tp = mix(tp, sf(sb1 + 8u), kp.y);
                }
                let b = laser_beam_ab(a, e, r, dens, o, d, t_end);
                if (b > 0.0) { c[i] = c[i] + power[i] * tp * b; }
            }
        }
    }
    return c;
}

// Macchie dei laser sul punto p, per lunghezza d'onda (interpolate come i fasci).
fn laser_spots_s(bs: Bsdf, col: vec4<f32>, wo: vec3<f32>, p: vec3<f32>, ng: vec3<f32>) -> vec4<f32> {
    var c = vec4<f32>(0.0);
    for (var li: u32 = 0u; li < pu(P_N_LASERS); li = li + 1u) {
        let lb = laser_base(li);
        let r = sf(lb + 3u);
        let power = illum4(sv3(lb));
        for (var i: u32 = 0u; i < 4u; i = i + 1u) {
            if (lam_pdf[i] <= 0.0) { continue; }
            let kp = laser_lambda_pair(lam[i]);
            let p0 = laser_path(li, 1u + u32(kp.x));
            let p1 = laser_path(li, 2u + u32(kp.x));
            let has0 = S[p0 + 1u] != 0u;
            let has1 = S[p1 + 1u] != 0u;
            var spot: vec3<f32>;
            var dir: vec3<f32>;
            var tp: f32;
            if (has0 && has1 && laser_compatible(p0, p1)) {
                spot = mix(sv3(p0 + 4u), sv3(p1 + 4u), kp.y);
                dir = normalize(mix(sv3(p0 + 8u), sv3(p1 + 8u), kp.y));
                tp = mix(sf(p0 + 12u), sf(p1 + 12u), kp.y);
            } else if (has0 && (kp.y < 0.5 || !has1)) {
                spot = sv3(p0 + 4u);
                dir = sv3(p0 + 8u);
                tp = sf(p0 + 12u);
            } else if (has1) {
                spot = sv3(p1 + 4u);
                dir = sv3(p1 + 8u);
                tp = sf(p1 + 12u);
            } else {
                continue;
            }
            let sp = laser_spot_at(spot, dir, r, p, ng);
            if (sp.w > 0.0) {
                let e = bsdf_eval_s(bs, col, wo, sp.xyz);
                c[i] = c[i] + e.f[i] * power[i] * tp * sp.w;
            }
        }
    }
    return c;
}

fn clamp_s(c: vec4<f32>, depth: u32, clamp_v: f32) -> vec4<f32> {
    let y = (c.x + c.y + c.z + c.w) * 0.25;
    if (clamp_v > 0.0 && depth > 0u && y > clamp_v) { return c * (clamp_v / y); }
    return c;
}

fn spectral_light_sample(n_lights: u32, n_select: u32, env_slot: u32, p: vec3<f32>) -> LSample {
    let li_idx = min(u32(rnd() * f32(n_select)), n_select - 1u);
    let u1 = rnd();
    let u2 = rnd();
    if (li_idx < n_lights) { return sample_light(li_idx, p, u1, u2); }
    if (li_idx == env_slot) { return env_light_sample(p, u1, u2); }
    return emissive_sample(p, rnd(), u1, u2);
}

fn radiance_spectral(o_in: vec3<f32>, d_in: vec3<f32>) -> vec4<f32> {
    media_n = 0u;
    var o = o_in;
    var d = d_in;
    var l = vec4<f32>(0.0);
    var beta = vec4<f32>(1.0);
    var specular = true;
    var camera_path = true;
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
        var hit = trace(o, d, 0.0, INF, false);
        for (var skip: u32 = 0u; skip < 8u && hit.found; skip = skip + 1u) {
            let hpb = pu(P_OFF_PRIMS) + hit.prim * PRIM_WORDS;
            let hm = S[hpb + 3u];
            let hng = sv3(hpb + 12u);
            let hfront = dot(hng, d) < 0.0;
            if (!media_false_hit(hm, hfront)) { break; }
            beta = beta * media_transmittance4(hit.t);
            media_cross(hm, hfront);
            var hside = hng;
            if (hfront) { hside = -hng; }
            o = offset_ray(surface_point(hit.prim, hit.u, hit.v), hside);
            hit = trace(o, d, 0.0, INF, false);
        }
        var t_surf = INF;
        if (hit.found) { t_surf = hit.t; }

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
        if (camera_path && pu(P_N_LASERS) > 0u) {
            var t_end = t_surf;
            if (lt > 0.0) { t_end = lt; }
            l = l + beta * laser_beams_s(o, d, t_end);
        }
        if (media_n > 0u) {
            var t_end = t_surf;
            if (lt > 0.0) { t_end = lt; }
            if (t_end < INF) { beta = beta * media_transmittance4(t_end); }
        }
        if (pu(P_N_FOGS) > 0u) {
            var t_end = t_surf;
            if (lt > 0.0) { t_end = lt; }
            let ev = fog_collision(o, d, t_end, camera_path);
            if (ev.x >= 0.0) {
                let fb = fog_base(u32(ev.y));
                let p = o + d * ev.x;
                beta = beta * albedo4(sv3(fb + 8u));
                if (all(beta == vec4<f32>(0.0))) { break; }
                let g = sf(fb + 11u);
                if (n_select > 0u) {
                    let ls = spectral_light_sample(n_lights, n_select, env_slot, p);
                    if (ls.ok) {
                        let ph = hg_pdf(g, dot(d, ls.wi));
                        let vis = light_visibility(p, ls);
                        if (vis > 0.0) {
                            var c: vec4<f32>;
                            if (ls.delta) {
                                c = illum4(ls.li) * (ph / sel_pdf);
                            } else {
                                let lp = ls.pdf * sel_pdf;
                                c = illum4(ls.li) * (ph * power_heuristic(lp, ph) / lp);
                            }
                            l = l + clamp_s(beta * c * vis, depth, clamp_v);
                        }
                    }
                }
                if ((S[fb + 15u] & 1u) != 0u) { break; }
                let wi = hg_sample(g, d, rnd(), rnd());
                prev_pdf = hg_pdf(g, dot(d, wi));
                specular = false;
                camera_path = false;
                o = p;
                d = wi;
                if (depth + 1u >= pu(P_RR_START)) {
                    let q = min(max(max(beta.x, beta.y), max(beta.z, beta.w)), 0.95);
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
                l = l + clamp_s(beta * illum4(lem) * w, depth, clamp_v);
            }
            break;
        }

        if (!hit.found) {
            var c = vec4<f32>(0.0);
            if (depth == 0u) {
                if ((pu(P_FLAGS) & 1u) != 0u) { c = beta * illum4(env_radiance(d, true)); }
            } else if (has_env) {
                var w = 1.0;
                if (!specular) { w = power_heuristic(prev_pdf, env_light_pdf(o, d) * sel_pdf); }
                c = beta * illum4(env_radiance(d, false)) * w;
            } else {
                c = beta * illum4(env_radiance(d, false));
            }
            l = l + clamp_s(c, depth, clamp_v);
            break;
        }

        let prim = hit.prim;
        let pb = pu(P_OFF_PRIMS) + prim * PRIM_WORDS;
        let p = surface_point(prim, hit.u, hit.v);
        let mat_id = S[pb + 3u];
        var m = load_mat(mat_id);
        var ng = sv3(pb + 12u);
        let front_face = dot(ng, d) < 0.0;
        if (m.transmission > 0.0) {
            // Indici alla hero; se uno dei due mezzi disperde resta solo la hero.
            let out_id = media_outside_id(mat_id, front_face);
            var n_out = 1.0;
            var out_disp = false;
            if (out_id >= 0) {
                n_out = mat_ior_hero(u32(out_id));
                out_disp = mat_word(u32(out_id), 34u) > 0.0;
            }
            if ((mat_word(mat_id, 34u) > 0.0 || out_disp) && !secondary_terminated()) {
                lam_pdf = vec4<f32>(lam_pdf.x * 0.25, 0.0, 0.0, 0.0);
                beta = vec4<f32>(beta.x, 0.0, 0.0, 0.0);
                l = vec4<f32>(l.x, 0.0, 0.0, 0.0);
            }
            m.ior = max(mat_ior_hero(mat_id) / n_out, 1e-3);
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
            l = l + clamp_s(beta * illum4(m.emission) * w, depth, clamp_v);
        }

        var bs: Bsdf;
        bs.frame = onb(ns);
        bs.ng = ng;
        bs.m = m;
        bs.entering = entering;
        let col = albedo4(m.base);

        if (pu(P_N_LASERS) > 0u && has_non_delta(bs)) {
            l = l + beta * laser_spots_s(bs, col, wo, p, sv3(pb + 12u));
        }

        if (n_select > 0u && has_non_delta(bs)) {
            let ls = spectral_light_sample(n_lights, n_select, env_slot, p);
            if (ls.ok) {
                let e = bsdf_eval_s(bs, col, wo, ls.wi);
                if (any(e.f != vec4<f32>(0.0))) {
                    let cosv = abs(dot(ls.wi, ns));
                    var side = ng;
                    if (dot(ls.wi, ng) < 0.0) { side = -ng; }
                    let vis = light_visibility(offset_ray(p, side), ls);
                    if (vis > 0.0) {
                        var c: vec4<f32>;
                        if (ls.delta) {
                            c = e.f * illum4(ls.li) * (cosv / sel_pdf);
                        } else {
                            let lp = ls.pdf * sel_pdf;
                            c = e.f * illum4(ls.li) * (cosv * power_heuristic(lp, e.pdf) / lp);
                        }
                        l = l + clamp_s(beta * c * vis, depth, clamp_v);
                    }
                }
            }
        }

        let ul = rnd();
        let u1 = rnd();
        let u2 = rnd();
        let s = bsdf_sample_s(bs, col, wo, ul, u1, u2);
        if (!s.ok) { break; }
        beta = beta * s.weight;
        if (!all(beta < vec4<f32>(INF)) || all(beta == vec4<f32>(0.0))) { break; }
        specular = s.delta;
        camera_path = camera_path && s.delta;
        prev_pdf = s.pdf;
        if (dot(s.wi, ng) < 0.0 && m.transmission > 0.0) { media_cross(mat_id, front_face); }
        var side = ng;
        if (dot(s.wi, ng) < 0.0) { side = -ng; }
        o = offset_ray(p, side);
        d = s.wi;

        if (depth + 1u >= pu(P_RR_START)) {
            let q = min(max(max(beta.x, beta.y), max(beta.z, beta.w)), 0.95);
            if (rnd() >= q) { break; }
            beta = beta / q;
        }
    }
    return l;
}

@compute @workgroup_size(8, 8, 1)
fn main_spectral(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w = pu(P_WIDTH);
    let h = pu(P_HEIGHT);
    if (gid.x >= w || gid.y >= h) { return; }
    let idx = gid.y * w + gid.x;
    if ((pu(P_FLAGS) & 2u) != 0u) {
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
        sample_wavelengths(rnd());
        let ndc_x = 2.0 * (f32(gid.x) + jx) / f32(w) - 1.0 + 2.0 * pf(P_SHIFT_X);
        let ndc_y = 1.0 - 2.0 * (f32(gid.y) + jy) / f32(h) + 2.0 * pf(P_SHIFT_Y);
        var dir = normalize(f + r * (ndc_x * th * aspect) + u * (ndc_y * th));
        var origin = cam_pos;
        let oh = pf(P_ORTHO_H);
        if (oh > 0.0) {
            dir = f;
            origin = cam_pos + r * (ndc_x * oh * aspect) + u * (ndc_y * oh) - f * pf(P_ORTHO_BACK);
        }
        let lr = pf(P_LENS_R);
        if (lr > 0.0) {
            let focus = cam_pos + dir * (pf(P_FOCUS) / max(dot(dir, f), 1e-6));
            let u0 = rnd();
            let u1 = rnd();
            let u2 = rnd();
            let a = sample_aperture(u0, u1, u2) * lr;
            origin = cam_pos + r * a.x + u * a.y;
            dir = normalize(focus - origin);
        }
        let c = spectral_to_rgb(radiance_spectral(origin, dir));
        if (c.x < INF && c.y < INF && c.z < INF && c.x > -INF && c.y > -INF && c.z > -INF && c.x == c.x && c.y == c.y && c.z == c.z) { sum = sum + c; }
    }
    OUT[idx] = vec4<f32>(sum, 0.0);
}
