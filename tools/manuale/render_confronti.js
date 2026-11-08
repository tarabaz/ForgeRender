// Render di confronto del manuale interno, fatti con forgerender-cli (GPU, 640×480).
//
//   cargo build --release -p forge_cli
//   node tools/manuale/render_confronti.js [filtro]
//
// Scrive i PNG in manuale/img/comune/ (li mostra il manuale nel programma) e una copia uguale in
// docs/manual/renders/. Nome = impostazione_cursore_valore (es. render_samples_064.png).
// Scene: la scena di prova del programma e la shader ball delle anteprime dei materiali.
const { execFileSync } = require("child_process");
const os = require("os");
const fs = require("fs");
const path = require("path");

const REPO = path.resolve(__dirname, "../..");
const CLI = path.join(REPO, "target/release/forgerender-cli" + (process.platform === "win32" ? ".exe" : ""));
const OUT = path.join(REPO, "manuale/img/comune");
const COPY = path.join(REPO, "docs/manual/renders");
const PAT = fs.mkdtempSync(path.join(os.tmpdir(), "forge-manuale-"));
fs.mkdirSync(OUT, { recursive: true });
fs.mkdirSync(COPY, { recursive: true });

const BALL = REPO + "/assets/anteprima/anteprima.frproject";
const base = ["--width", "640", "--height", "480", "--backend", "gpu", "--quiet", "--seed", "7"];

// Camera della scena di prova.
const camPos = [0.3, 1.6, 4.2], camTgt = [0.0, 0.35, 0.0];
const vfovOf = (mm) => (2 * Math.atan(12 / mm) * 180) / Math.PI;
const baseFocal = 12 / Math.tan((38 * Math.PI) / 360); // ≈ 34.9 mm
function camAtFocal(mm) {
  const d = camPos.map((v, i) => v - camTgt[i]);
  const k = mm / baseFocal;
  return { position: d.map((v, i) => camTgt[i] + v * k), target: camTgt, vfov_deg: vfovOf(mm) };
}
// Camera della shader ball, un po' più indietro per vedere tutta la base.
const ballTgt = [-0.0000034, 0.041023, -0.0000034];
const ballPos0 = [0.08195308, 0.08959325, 0.11557515];
const ballCam = { position: ballPos0.map((v, i) => ballTgt[i] + (v - ballTgt[i]) * 1.3), target: ballTgt };

function kelvin(k) {
  const t = Math.min(Math.max(k, 1000), 40000) / 100;
  const r = t <= 66 ? 255 : 329.69873 * Math.pow(t - 60, -0.13320476);
  const g = t <= 66 ? 99.4708 * Math.log(t) - 161.11957 : 288.12217 * Math.pow(t - 60, -0.07551485);
  const b = t >= 66 ? 255 : t <= 19 ? 0 : 138.51773 * Math.log(t - 10) - 305.0448;
  const lin = [r, g, b].map((v) => {
    const c = Math.min(Math.max(v / 255, 0), 1);
    return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  });
  const m = Math.max(...lin, 1e-6);
  return lin.map((v) => v / m);
}

// Luci della scena di prova (preset "studio" con centro (0, 0.5, 0) e raggio 1.2).
const C = [0, 0.5, 0], R = 1.2;
const at = (x, y, z) => [C[0] + x * R, C[1] + y * R, C[2] + z * R];
function studio({ keySize = 2.5 * R, color = [1, 1, 1], keyMul = 1, fill = 0.8 } = {}) {
  const keyRef = 2.5 * R;
  return [
    {
      name: "Softbox principale", kind: { Area: { width: keySize, height: keySize } },
      position: at(-2, 3, 2.5), target: C, color, intensity: 3.5 * keyMul * (keyRef * keyRef) / (keySize * keySize),
      cast_shadows: true, visible_to_camera: false,
    },
    {
      name: "Riempimento", kind: { Area: { width: 2 * R, height: 2 * R } },
      position: at(3, 1.5, 1.5), target: C, color, intensity: fill, cast_shadows: true, visible_to_camera: false,
    },
  ];
}
const gradient = (strength) => ({
  Gradient: { zenith: [0.35, 0.42, 0.55], horizon: [0.8, 0.82, 0.85], ground: [0.25, 0.24, 0.23], strength },
});

const ball = (mat, extra = {}) => ({
  scene: "ball",
  patch: { camera: ballCam, settings: { noise_threshold: 0, denoise: { strength: 1.0 } }, materials: { ceramica: mat }, ...extra },
});
const test = (patch = {}, args = []) => ({ scene: "test", patch, args });
const fmt = (v) => String(v).replace(".", "");

const jobs = {};
// --- Render: campioni (senza denoiser, per vedere il rumore vero) ---
for (const n of [16, 64, 256, 1024]) jobs[`render_samples_${String(n).padStart(3, "0")}`] = test({}, ["--spp", n, "--denoise", "off"]);
// --- Render: denoiser ---
jobs["render_denoiser_off"] = test({}, ["--spp", 256, "--denoise", "off"]);
jobs["render_denoiser_on"] = test({}, ["--spp", 256, "--denoise", "finale"]);
// --- Render: rimbalzi ---
for (const b of [1, 3, 12]) jobs[`render_bounces_${String(b).padStart(2, "0")}`] = test({}, ["--spp", 256, "--bounces", b]);
// --- Render: esposizione ---
for (const [n, ev] of [["minus1", -1], ["0", 0], ["plus1", 1]]) jobs[`render_exposure_${n}`] = test({ settings: { exposure_ev: ev } }, ["--spp", 256]);
// --- Render: tone mapping ---
jobs["render_tonemapping_standard"] = test({ settings: { tone_mapping: "Standard", exposure_ev: 0.5 } }, ["--spp", 256]);
jobs["render_tonemapping_aces"] = test({ settings: { tone_mapping: "AcesFilmic", exposure_ev: 0.5 } }, ["--spp", 256]);
// --- Render: soglia di rumore (campionamento adattivo), massimo 1024 campioni, senza denoiser ---
for (const t of [0.08, 0.04, 0.01]) jobs[`render_noisethreshold_${t.toFixed(3).replace(".", "")}`] = test({}, ["--spp", 1024, "--noise-threshold", t, "--adaptive-min", 16, "--denoise", "off"]);
// --- Render: stili ---
jobs["render_style_photoreal"] = test({}, ["--spp", 256]);
jobs["render_style_toon"] = test({}, ["--spp", 256, "--style", "fumetto"]);
jobs["render_style_technical"] = test({}, ["--spp", 256, "--style", "tecnico"]);
jobs["render_outlines_on"] = test({}, ["--spp", 256, "--outlines"]);
// --- Render: caustiche ---
const pointLight = [{ name: "Lampadina", kind: "Point", position: [-0.5, 2.1, -0.5], target: [0, 0.45, 0.8], color: [1, 0.97, 0.92], intensity: 6, cast_shadows: true, visible_to_camera: false }];
jobs["render_caustics_off"] = test({ lights: pointLight, environment: gradient(0.1) }, ["--spp", 512]);
jobs["render_caustics_on"] = test({ lights: pointLight, environment: gradient(0.1), settings: { caustics: { enabled: true, photons_k: 400 } } }, ["--spp", 512]);
// --- Materiali (shader ball) ---
const gold = [1.0, 0.71, 0.29];
for (const r of [0.05, 0.3, 0.7]) jobs[`mat_roughness_${fmt(r.toFixed(2))}`] = ball({ base_color: gold, metallic: 1, roughness: r, transmission: 0 });
for (const m of [0, 0.5, 1]) jobs[`mat_metallic_${fmt(m.toFixed(2))}`] = ball({ base_color: [0.9, 0.42, 0.25], metallic: m, roughness: 0.2, transmission: 0 });
for (const s of [0, 0.5, 1]) jobs[`mat_specular_${fmt(s.toFixed(2))}`] = ball({ base_color: [0.02, 0.02, 0.025], metallic: 0, roughness: 0.15, specular: s, transmission: 0 });
for (const i of [1.1, 1.5, 2.4]) jobs[`mat_ior_${fmt(i.toFixed(2))}`] = ball({ base_color: [1, 1, 1], metallic: 0, roughness: 0, ior: i, transmission: 1 });
for (const t of [0, 0.8, 1]) jobs[`mat_transmission_${fmt(t.toFixed(2))}`] = ball({ base_color: [0.55, 0.78, 0.95], metallic: 0, roughness: 0.05, ior: 1.5, transmission: t });
// --- Luci ---
for (const [n, s] of [["015", 0.15], ["080", 0.8], ["300", 3.0]]) jobs[`light_size_${n}`] = test({ lights: studio({ keySize: s, fill: 0.15, keyMul: 1.6 }), environment: gradient(0.12) }, ["--spp", 256]);
for (const k of [2700, 5500, 9000]) jobs[`light_temperature_${k}`] = test({ lights: studio({ color: kelvin(k) }) }, ["--spp", 256]);
// --- Ambiente ---
for (const [n, s] of [["010", 0.1], ["045", 0.45], ["150", 1.5]]) jobs[`env_strength_${n}`] = test({ environment: gradient(s) }, ["--spp", 256]);
// --- Camera ---
for (const mm of [24, 50, 135]) jobs[`cam_focal_${String(mm).padStart(3, "0")}`] = test({ camera: camAtFocal(mm) }, ["--spp", 256]);
const torus = [1.2, 0.14, 0.2];
const dir = [1.6, 0.35, 0.75]; const dl = Math.hypot(...dir);
const dofPos = torus.map((v, i) => v + (dir[i] / dl) * 1.45);
const dofTgt = [torus[0] - 0.55, 0.3, torus[2] - 0.3];
const dofFocus = 1.45;
for (const [n, f] of [["12", 1.2], ["40", 4], ["160", 16]]) jobs[`cam_fstop_${n}`] = test({ camera: { position: dofPos, target: dofTgt, vfov_deg: vfovOf(50), lens: { dof: true, f_stop: f, focus_distance: dofFocus, blades: 0 } } }, ["--spp", 256]);
// --- Effetti: foschia ---
for (const [n, d] of [["low", 0.3], ["mid", 1.0], ["high", 3.0]]) jobs[`fx_haze_density_${n}`] = test({}, ["--spp", 256, "--haze", "nebbia-bassa", "--haze-density", d, "--dark", "--spot"]);

const filter = process.argv[2];
for (const [name, j] of Object.entries(jobs)) {
  if (filter && !name.includes(filter)) continue;
  const out = path.join(OUT, name + ".png");
  const args = ["render", ...(j.scene === "ball" ? ["--project", BALL] : ["--test-scene"]), ...base];
  const extra = (j.args || []).map(String);
  if (!extra.includes("--spp")) args.push("--spp", "256");
  if (!extra.includes("--denoise")) args.push("--denoise", "finale");
  args.push(...extra);
  if (j.patch && Object.keys(j.patch).length) {
    const pf = path.join(PAT, name + ".json");
    fs.writeFileSync(pf, JSON.stringify(j.patch, null, 1));
    args.push("--patch", pf);
  }
  args.push("--out", out);
  const t0 = Date.now();
  try {
    execFileSync(CLI, args, { cwd: REPO, stdio: ["ignore", "pipe", "pipe"] });
    fs.copyFileSync(out, path.join(COPY, name + ".png"));
    console.log(`${name}: ${((Date.now() - t0) / 1000).toFixed(1)} s`);
  } catch (e) {
    console.log(`${name}: ERRORE ${e.stderr}`);
  }
}
