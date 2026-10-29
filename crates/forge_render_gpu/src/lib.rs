//! Backend GPU di ForgeRender: path tracer in compute shader WGSL eseguito con
//! wgpu (Vulkan o DirectX 12 su Windows, Vulkan su Linux).
//!
//! - Lo stesso algoritmo del backend CPU (stesse BSDF, MIS, HDRI, emissivi,
//!   texture), con un generatore casuale diverso: le immagini convergono allo
//!   stesso risultato ma non sono identiche bit per bit.
//! - Tutta la scena sta in un solo storage buffer ("bindless", vedi `layout`).
//! - Ogni chiamata calcola N campioni per pixel; N si adatta per tenere ogni
//!   passata attorno a [`TARGET_PASS_MS`], così pausa e stop restano reattivi e
//!   non si rischia il timeout del driver (TDR) di Windows.

pub mod layout;

use anyhow::{anyhow, Context};
use forge_render_core::adaptive::TileMask;
use forge_render_core::{RenderBackend, RenderScene, RenderSettings};
use layout::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

const SHADER: &str = include_str!("path_tracer.wgsl");
/// Durata obiettivo di una passata GPU.
pub const TARGET_PASS_MS: f64 = 60.0;
const MAX_SAMPLES_PER_PASS: u32 = 64;

/// Informazioni sul dispositivo scelto.
#[derive(Clone, Debug)]
pub struct GpuInfo {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub driver: String,
}

struct SceneGpu {
    scene_id: u64,
    buffer: wgpu::Buffer,
    params: [u32; PARAM_WORDS],
    bytes: u64,
}

struct Target {
    size: (u32, u32),
    out: wgpu::Buffer,
    readback: wgpu::Buffer,
}

pub struct GpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    scene: Option<SceneGpu>,
    target: Option<Target>,
    info: GpuInfo,
    samples_per_pass: u32,
    /// Maschera del campionamento adattivo (un bit per riquadro), almeno una parola.
    mask: wgpu::Buffer,
}

fn mask_buffer(device: &wgpu::Device, words: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("forge-gpu-mask"),
        size: (words.max(1) * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn is_warp(info: &wgpu::AdapterInfo) -> bool {
    info.backend == wgpu::Backend::Dx12 && info.device_type == wgpu::DeviceType::Cpu
}

impl GpuBackend {
    /// Crea il backend sulla GPU ad alte prestazioni (la scheda dedicata invece
    /// della grafica integrata). Restituisce errore se non c'è una GPU adatta.
    pub fn new() -> anyhow::Result<Self> {
        // Ordine dei backend: Vulkan (ottimo sui driver NVIDIA), poi DirectX 12, poi gli altri.
        // `FORGE_GPU_BACKEND=vulkan|dx12` forza una scelta.
        let order: Vec<wgpu::Backends> = match std::env::var("FORGE_GPU_BACKEND").ok().as_deref() {
            Some("vulkan") => vec![wgpu::Backends::VULKAN],
            Some("dx12") => vec![wgpu::Backends::DX12],
            _ => vec![wgpu::Backends::VULKAN, wgpu::Backends::DX12, wgpu::Backends::PRIMARY],
        };
        let mut found = None;
        let mut last_err = String::from("nessun backend disponibile");
        for backends in order {
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
            desc.backends = backends;
            let instance = wgpu::Instance::new(desc);
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                ..Default::default()
            })) {
                // WARP (DirectX 12 in software) va in crash con questo shader: si scarta,
                // a meno che `FORGE_GPU_ALLOW_WARP=1`. Lavapipe (Vulkan in software) resta valido.
                Ok(a) if is_warp(&a.get_info()) && std::env::var_os("FORGE_GPU_ALLOW_WARP").is_none() => {
                    last_err = "solo DirectX 12 in software (WARP), nessuna GPU reale".into()
                }
                Ok(a) if a.get_info().backend != wgpu::Backend::Gl => {
                    found = Some(a);
                    break;
                }
                Ok(_) => last_err = "solo OpenGL disponibile".into(),
                Err(e) => last_err = e.to_string(),
            }
        }
        let adapter = found.ok_or_else(|| anyhow!("nessuna GPU compatibile: {last_err}"))?;
        let ai = adapter.get_info();
        let info = GpuInfo {
            name: ai.name.clone(),
            backend: format!("{:?}", ai.backend),
            device_type: format!("{:?}", ai.device_type),
            driver: format!("{} {}", ai.driver, ai.driver_info).trim().to_string(),
        };
        if ai.backend == wgpu::Backend::Gl {
            return Err(anyhow!(
                "la GPU è disponibile solo tramite OpenGL, che non supporta i compute shader necessari"
            ));
        }
        // Buffer della scena: si chiede il massimo consentito dalla scheda.
        let al = adapter.limits();
        let limits = wgpu::Limits {
            max_storage_buffer_binding_size: al.max_storage_buffer_binding_size,
            max_buffer_size: al.max_buffer_size,
            ..wgpu::Limits::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("forge-gpu"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            ..Default::default()
        }))
        .context("creazione del dispositivo GPU")?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("path_tracer.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("forge-gpu-bgl"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    2,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    3,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("forge-gpu-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("forge-gpu-pipeline"),
            layout: Some(&layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-gpu-params"),
            size: (PARAM_WORDS * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        tracing::info!(
            "backend GPU: {} ({}, {}), driver {}",
            info.name,
            info.backend,
            info.device_type,
            info.driver
        );
        let mask = mask_buffer(&device, 1);
        Ok(Self {
            mask,
            device,
            queue,
            pipeline,
            bgl,
            uniform,
            scene: None,
            target: None,
            info,
            samples_per_pass: 1,
        })
    }

    pub fn info(&self) -> &GpuInfo {
        &self.info
    }

    fn upload_scene(&mut self, scene: &RenderScene) -> anyhow::Result<()> {
        if self.scene.as_ref().is_some_and(|s| s.scene_id == scene.content_id) {
            return Ok(());
        }
        let t0 = Instant::now();
        let sb = layout::build(scene);
        let bytes = (sb.words.len() * 4) as u64;
        let limit = self.device.limits().max_storage_buffer_binding_size;
        if bytes > limit {
            return Err(anyhow!(
                "scena troppo grande per la memoria della GPU in un solo buffer ({} MB, limite {} MB): usa il backend CPU",
                bytes / 1_000_000,
                limit / 1_000_000
            ));
        }
        use wgpu::util::DeviceExt;
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forge-gpu-scene"),
            contents: bytemuck::cast_slice(&sb.words),
            usage: wgpu::BufferUsages::STORAGE,
        });
        tracing::info!(
            "scena caricata sulla GPU: {:.1} MB in {:.0} ms",
            bytes as f64 / 1e6,
            t0.elapsed().as_secs_f64() * 1e3
        );
        self.scene = Some(SceneGpu {
            scene_id: scene.content_id,
            buffer,
            params: sb.params,
            bytes,
        });
        Ok(())
    }

    fn ensure_target(&mut self, w: u32, h: u32) {
        if self.target.as_ref().is_some_and(|t| t.size == (w, h)) {
            return;
        }
        let size = w as u64 * h as u64 * 16;
        let out = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-gpu-out"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-gpu-readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.target = Some(Target {
            size: (w, h),
            out,
            readback,
        });
    }

    /// Memoria video occupata dalla scena corrente (byte).
    pub fn scene_bytes(&self) -> u64 {
        self.scene.as_ref().map(|s| s.bytes).unwrap_or(0)
    }
}

impl RenderBackend for GpuBackend {
    fn name(&self) -> String {
        "GPU path tracer".into()
    }

    fn device_info(&self) -> String {
        format!(
            "{} · {} · {} campioni/passata",
            self.info.name, self.info.backend, self.samples_per_pass
        )
    }

    fn render_pass(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
    ) -> anyhow::Result<u32> {
        self.pass(scene, settings, first_sample, max_samples, out, cancel, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_pass_masked(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
        mask: &TileMask,
    ) -> anyhow::Result<u32> {
        self.pass(scene, settings, first_sample, max_samples, out, cancel, Some(mask))
    }
}

impl GpuBackend {
    /// Una passata di `n` campioni per pixel (solo i riquadri attivi se c'è la maschera).
    #[allow(clippy::too_many_arguments)]
    fn pass(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
        mask: Option<&TileMask>,
    ) -> anyhow::Result<u32> {
        let (w, h) = (settings.width, settings.height);
        anyhow::ensure!(out.len() == (w * h) as usize, "buffer di uscita di dimensione errata");
        if cancel.load(Ordering::Relaxed) {
            return Ok(0);
        }
        self.upload_scene(scene)?;
        self.ensure_target(w, h);
        // La prima passata di un render è di un solo campione: muovendo la vista l'immagine
        // nuova compare subito, poi le passate crescono fino a ~TARGET_PASS_MS.
        let n = if first_sample == 0 {
            1
        } else {
            self.samples_per_pass.clamp(1, max_samples.max(1))
        };

        let sg = self.scene.as_ref().expect("scena");
        let mut p = sg.params;
        let cam = &scene.camera;
        let (r, u, f) = cam.basis();
        let fv = |p: &mut [u32; PARAM_WORDS], i: usize, v: f32| p[i] = v.to_bits();
        p[P_WIDTH] = w;
        p[P_HEIGHT] = h;
        p[P_FIRST_SAMPLE] = first_sample;
        p[P_N_SAMPLES] = n;
        p[P_SEED_LO] = settings.seed as u32;
        p[P_SEED_HI] = (settings.seed >> 32) as u32;
        p[P_MAX_BOUNCES] = settings.max_bounces;
        p[P_RR_START] = settings.russian_roulette_start;
        p[P_FLAGS] = settings.environment_visible as u32;
        if let Some(m) = mask {
            let bits = m.bits();
            if (self.mask.size() as usize) < bits.len() * 4 {
                self.mask = mask_buffer(&self.device, bits.len());
            }
            self.queue.write_buffer(&self.mask, 0, bytemuck::cast_slice(&bits));
            p[P_FLAGS] |= 2;
            p[P_TILES_X] = m.tiles_x;
        }
        fv(&mut p, P_CLAMP, settings.clamp_indirect);
        for k in 0..3 {
            fv(&mut p, P_CAM_POS + k, cam.position[k]);
            fv(&mut p, P_CAM_R + k, r[k]);
            fv(&mut p, P_CAM_U + k, u[k]);
            fv(&mut p, P_CAM_F + k, f[k]);
        }
        fv(&mut p, P_TAN_HALF, (cam.vfov_deg.to_radians() * 0.5).tan());
        fv(&mut p, P_ASPECT, w as f32 / h as f32);
        fv(&mut p, P_LENS_R, cam.lens_radius());
        fv(&mut p, P_FOCUS, cam.lens.focus_distance.max(1e-4));
        p[P_BLADES] = cam.lens.blades;
        fv(&mut p, P_BLADE_ROT, cam.lens.blade_rotation_deg.to_radians());
        fv(&mut p, P_SHIFT_X, cam.lens.shift_x);
        fv(&mut p, P_SHIFT_Y, cam.lens.shift_y);
        self.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&p));

        let t = self.target.as_ref().expect("target");
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forge-gpu-bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sg.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: t.out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.mask.as_entire_binding(),
                },
            ],
        });
        let t0 = Instant::now();
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("forge-gpu-pass"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("path-trace"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
        }
        enc.copy_buffer_to_buffer(&t.out, 0, &t.readback, 0, w as u64 * h as u64 * 16);
        self.queue.submit(Some(enc.finish()));

        let slice = t.readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|e| anyhow!("errore della GPU durante il rendering: {e}"))?;
        rx.recv()
            .context("lettura dei risultati dalla GPU")?
            .map_err(|e| anyhow!("lettura dei risultati dalla GPU: {e}"))?;
        {
            let data = slice
                .get_mapped_range()
                .map_err(|e| anyhow!("lettura dei risultati dalla GPU: {e:?}"))?;
            let px: &[[f32; 4]] = bytemuck::cast_slice(&data);
            for (o, v) in out.iter_mut().zip(px) {
                *o = [v[0], v[1], v[2]];
            }
        }
        t.readback.unmap();

        // Adatta i campioni per passata alla durata misurata.
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let per_sample = ms / n as f64;
        let ideal = (TARGET_PASS_MS / per_sample.max(1e-3)).floor() as u32;
        self.samples_per_pass = ideal.clamp(1, MAX_SAMPLES_PER_PASS).min(self.samples_per_pass * 2).max(1);
        // Una passata finita è valida anche se nel frattempo è arrivato un comando: si tiene,
        // così trascinando la vista si vede sempre l'ultima immagine calcolata invece di nulla.
        Ok(n)
    }
}

/// Backend GPU condiviso: il dispositivo e la pipeline (costosi da creare) vengono
/// riutilizzati tra un rendering e l'altro.
#[derive(Clone)]
pub struct SharedGpu(pub std::sync::Arc<std::sync::Mutex<GpuBackend>>);

impl SharedGpu {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(std::sync::Arc::new(std::sync::Mutex::new(GpuBackend::new()?))))
    }

    pub fn info(&self) -> GpuInfo {
        self.0.lock().map(|g| g.info().clone()).unwrap_or(GpuInfo {
            name: "?".into(),
            backend: String::new(),
            device_type: String::new(),
            driver: String::new(),
        })
    }
}

impl RenderBackend for SharedGpu {
    fn name(&self) -> String {
        "GPU path tracer".into()
    }

    fn device_info(&self) -> String {
        self.0.lock().map(|g| g.device_info()).unwrap_or_default()
    }

    fn render_pass(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
    ) -> anyhow::Result<u32> {
        let mut g = self
            .0
            .lock()
            .map_err(|_| anyhow!("backend GPU non disponibile (errore precedente)"))?;
        g.render_pass(scene, settings, first_sample, max_samples, out, cancel)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_pass_masked(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
        mask: &TileMask,
    ) -> anyhow::Result<u32> {
        let mut g = self
            .0
            .lock()
            .map_err(|_| anyhow!("backend GPU non disponibile (errore precedente)"))?;
        g.render_pass_masked(scene, settings, first_sample, max_samples, out, cancel, mask)
    }
}
