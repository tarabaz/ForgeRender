# Spectral rendering

🇮🇹 [Leggi in italiano](SPECTRAL.it.md)

ForgeRender has two light-transport modes, chosen in **Render → Colour**:

| Mode | What it does | When to use it |
|---|---|---|
| **RGB** (default) | Three channels (red, green, blue). Unchanged, fastest. | Almost always. |
| **Spectral** | Four wavelengths per path (hero wavelength). Dispersion in glass and gems, more faithful colours under saturated lights. | Glass, crystal, jewellery, prisms; about 2× slower and with more colour noise, so it needs more samples. |

Spectral is an option: projects open in RGB unless they were saved in spectral mode. Both CPU and GPU backends support it, and they produce the same result.

## Dispersion

Transparent materials have a **Dispersion** setting (Abbe number V, 0 = none). Lower V = colours spread further apart. Presets: crown glass BK7 (V 64.2, IOR 1.517), flint (36.4, 1.62), lead crystal (30, 1.7), polycarbonate (30, 1.586), diamond (55, 2.417). The index of refraction follows Cauchy's equation n(λ) = A + B/λ², derived from the IOR at the d line (587.6 nm) and V. Dispersion is visible only in spectral mode.

## How it works

- **Wavelengths**: four per path in 360–830 nm, sampled with PBRT v4's visible-wavelength distribution (pdf ∝ 1/cosh²(0.0072 (λ − 538))), stratified at offsets of ¼ (Wilkie et al. 2014, *Hero Wavelength Spectral Sampling*).
- **Directions do not depend on wavelength** (all four share the same sampling pdfs and MIS weights), except at dispersive dielectrics. There the hero wavelength's IOR is used and the other three are terminated, with the hero's pdf divided by 4 (PBRT v4 `TerminateSecondary`). The estimator stays unbiased.
- **RGB → spectrum**: scene colours stay RGB until they are used, then become smooth spectra s(c₀t² + c₁t + c₂) with s(x) = ½ + x/(2√(1+x²)) (Jakob & Hanika 2019, as in PBRT v4). The 3 × 64³ coefficient table is computed once per process by our own Gauss–Newton fit in CIELAB, about 2 s on a dedicated thread pool; no external data files. Reflectances are bounded to [0, 1]; emission uses the 2·max scale.
- **Colour**: CIE 1931 2° matching functions (analytic fit by Wyman, Sloan & Shirley 2013), XYZ → linear sRGB primaries with a Bradford adaptation from the equal-energy white. A constant spectrum maps exactly to (1, 1, 1), so a white light stays white, as in RGB mode. PBRT instead multiplies lights by D65; here light colours are already expressed in the scene's RGB space.
- **Volumes**: Beer–Lambert per wavelength, T(λ) = R(λ)^(d/d₀), with R the spectrum of the volume colour at depth d₀.
- **Accumulation**: each sample is converted to RGB before accumulation. The conversion is linear, so this equals accumulating XYZ, and accumulation, denoiser, tone mapping and EXR export are unchanged.
- **Code**: `forge_render_core::spectral` (wavelengths, CMFs, table, Cauchy), `forge_render_cpu::spectral` (integrator; the BSDF is generic over `Vec3`/`Vec4` channels, so the RGB build is unchanged), `forge_render_gpu/src/path_tracer_spectral.wgsl` (`main_spectral` entry point, data in binding 4, loaded only in spectral mode).

## Validation

Automated tests (`cargo test`):
- visible pdf integrates to 1 and matches PBRT's constant; sampling follows the CDF;
- RGB → spectrum → RGB round trip: maximum error 0.0004, including pure primaries;
- terminating secondaries keeps a white spectrum white on average;
- white furnace (CPU and GPU): diffuse, glass, dispersive glass and rough glass stay at 1;
- area light over a diffuse plane matches the analytic form factor within 3% (MIS);
- spectral matches RGB on single-bounce coloured scenes;
- dispersion separates colours (mean saturation 0.03 → 0.16), and RGB ignores the Abbe number;
- GPU vs CPU spectral on the test scene: RMSE equal to CPU–CPU noise (0.024 vs 0.025).

Measured on an RTX 5070, 800 × 450, 2048 spp: RGB 7.4 s, spectral 14.0 s.

## Not done yet

Thin-film interference (Airy), measured conductor spectra (η, k), blackbody light spectra, and spectral EXR output. These are possible next steps from the plan.
