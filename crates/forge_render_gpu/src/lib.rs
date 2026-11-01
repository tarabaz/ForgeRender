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

/// Path tracer RGB e, in coda, la variante spettrale (punto d'ingresso `main_spectral`).
const SHADER: &str = concat!(include_str!("path_tracer.wgsl"), "\n", include_str!("path_tracer_spectral.wgsl"));
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
    /// Tracciamento dei raggi con gli RT core (ray query).
    pub rt_cores: bool,
}

struct SceneGpu {
    scene_id: u64,
    buffer: wgpu::Buffer,
    params: [u32; PARAM_WORDS],
    bytes: u64,
    /// Strutture di accelerazione per gli RT core (se disponibili e se la scena ci sta).
    accel: Option<SceneAccel>,
}

/// Triangoli della scena nelle strutture di accelerazione hardware: un BLAS con tutti i triangoli
/// (nell'ordine delle primitive) in un TLAS con una sola istanza.
struct SceneAccel {
    tlas: wgpu::Tlas,
    _blas: wgpu::Blas,
    _vertices: wgpu::Buffer,
}

/// Pipeline con le ray query (RT core): layout con il binding 5 (TLAS), shader con `trace` in
/// hardware.
struct RtPipelines {
    bgl: wgpu::BindGroupLayout,
    layout: wgpu::PipelineLayout,
    module: wgpu::ShaderModule,
    main: wgpu::ComputePipeline,
    spectral: Option<wgpu::ComputePipeline>,
}

/// Shader con le ray query: lo stesso path tracer, con il `trace` della BVH rinominato e quello
/// hardware al suo posto.
fn rt_shader_source() -> String {
    let base = SHADER.replacen("fn trace(o: vec3<f32>", "fn trace_bvh(o: vec3<f32>", 1);
    format!("enable wgpu_ray_query;\n{base}\n{}", include_str!("path_tracer_rt.wgsl"))
}

/// Ray query volute? `FORGE_GPU_RT=0` le spegne (si usa la BVH dello shader).
fn rt_wanted() -> bool {
    std::env::var("FORGE_GPU_RT").map(|v| v != "0").unwrap_or(true)
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
    /// Per creare la pipeline spettrale alla prima richiesta.
    module: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    /// Render spettrale: pipeline e dati (tabella RGB → spettro), creati alla prima richiesta.
    spectral: Option<(wgpu::ComputePipeline, wgpu::Buffer)>,
    /// Buffer minimo al posto dei dati spettrali nel render RGB.
    spectral_dummy: wgpu::Buffer,
    /// RT core (ray query): `None` se la scheda o il driver non le hanno, o se spente.
    rt: Option<RtPipelines>,
}

/// Shader e pipeline con le ray query; `None` (si resta sulla BVH) se la validazione fallisce.
fn create_rt(device: &wgpu::Device, base_entries: &[wgpu::BindGroupLayoutEntry]) -> Option<RtPipelines> {
    let t0 = Instant::now();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut entries = base_entries.to_vec();
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 5,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::AccelerationStructure { vertex_return: false },
        count: None,
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("forge-gpu-bgl-rt"),
        entries: &entries,
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("forge-gpu-layout-rt"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("path_tracer_rt.wgsl"),
        source: wgpu::ShaderSource::Wgsl(rt_shader_source().into()),
    });
    let main = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("forge-gpu-pipeline-rt"),
        layout: Some(&layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        tracing::warn!("ray query non utilizzabili, si usa la BVH dello shader: {e}");
        return None;
    }
    tracing::info!("pipeline RT pronta in {:.0} ms", t0.elapsed().as_secs_f64() * 1e3);
    Some(RtPipelines {
        bgl,
        layout,
        module,
        main,
        spectral: None,
    })
}

/// Strutture di accelerazione hardware con i triangoli della scena (nell'ordine delle primitive).
/// `None` se la scena è vuota o supera i limiti della scheda (si usa la BVH dello shader).
fn build_accel(device: &wgpu::Device, queue: &wgpu::Queue, scene: &RenderScene) -> Option<SceneAccel> {
    let geo = &scene.geometry;
    let n = geo.prim_to_slot.len();
    if n == 0 || n as u64 > device.limits().max_blas_primitive_count as u64 {
        return None;
    }
    let t0 = Instant::now();
    let mut verts: Vec<f32> = Vec::with_capacity(n * 9);
    for prim in 0..n {
        for v in geo.vertices(prim) {
            verts.extend_from_slice(&v.to_array());
        }
    }
    use wgpu::util::DeviceExt;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("forge-gpu-blas-vertici"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::BLAS_INPUT,
    });
    let size = wgpu::BlasTriangleGeometrySizeDescriptor {
        vertex_format: wgpu::VertexFormat::Float32x3,
        vertex_count: (n * 3) as u32,
        index_format: None,
        index_count: None,
        flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
    };
    let blas = device.create_blas(
        &wgpu::CreateBlasDescriptor {
            label: Some("forge-gpu-blas"),
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        },
        wgpu::BlasGeometrySizeDescriptors::Triangles {
            descriptors: vec![size.clone()],
        },
    );
    let mut tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
        label: Some("forge-gpu-tlas"),
        max_instances: 1,
        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
    });
    tlas[0] = Some(wgpu::TlasInstance::new(
        &blas,
        [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        0,
        0xff,
    ));
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("forge-gpu-accel"),
    });
    enc.build_acceleration_structures(
        std::iter::once(&wgpu::BlasBuildEntry {
            blas: &blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                size: &size,
                vertex_buffer: &vertices,
                first_vertex: 0,
                vertex_stride: 12,
                index_buffer: None,
                first_index: None,
                transform_buffer: None,
                transform_buffer_offset: None,
            }]),
        }),
        std::iter::once(&tlas),
    );
    queue.submit(Some(enc.finish()));
    if let Some(e) = pollster::block_on(scope.pop()) {
        tracing::warn!("strutture RT non costruite, si usa la BVH dello shader: {e}");
        return None;
    }
    tracing::info!("strutture RT: {} triangoli in {:.0} ms", n, t0.elapsed().as_secs_f64() * 1e3);
    Some(SceneAccel {
        tlas,
        _blas: blas,
        _vertices: vertices,
    })
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
    /// Usa gli RT core (ray query) se ci sono, salvo `FORGE_GPU_RT=0`.
    pub fn new() -> anyhow::Result<Self> {
        Self::with_rt(rt_wanted())
    }

    /// Come [`GpuBackend::new`], scegliendo se provare le ray query (RT core). Senza supporto o
    /// se la loro preparazione fallisce si usa la BVH dello shader.
    pub fn with_rt(want_rt: bool) -> anyhow::Result<Self> {
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
            rt_cores: false,
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
        // RT core: ray query di wgpu (sperimentali, solo Vulkan).
        let rt_supported = want_rt && adapter.features().contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
        let request = |rt: bool| {
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("forge-gpu"),
                required_features: if rt {
                    wgpu::Features::EXPERIMENTAL_RAY_QUERY
                } else {
                    wgpu::Features::empty()
                },
                required_limits: if rt {
                    limits.clone().using_acceleration_structure_values(al.clone())
                } else {
                    limits.clone()
                },
                // SAFETY: le ray query di wgpu sono sperimentali; si usano solo per il tracciamento
                // dei raggi, con lo stesso risultato della BVH (verificato dai test GPU) e con il
                // ritorno automatico alla BVH se la preparazione fallisce.
                experimental_features: if rt {
                    unsafe { wgpu::ExperimentalFeatures::enabled() }
                } else {
                    wgpu::ExperimentalFeatures::disabled()
                },
                ..Default::default()
            }))
        };
        let (device, queue, rt_device) = match rt_supported.then(|| request(true)) {
            Some(Ok((d, q))) => (d, q, true),
            Some(Err(e)) => {
                tracing::warn!("ray query non disponibili ({e}): si usa la BVH dello shader");
                let (d, q) = request(false).context("creazione del dispositivo GPU")?;
                (d, q, false)
            }
            None => {
                let (d, q) = request(false).context("creazione del dispositivo GPU")?;
                (d, q, false)
            }
        };
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
        let base_entries = [
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
            // Dati del render spettrale (nel render RGB un buffer minimo).
            entry(
                4,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
        ];
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("forge-gpu-bgl"),
            entries: &base_entries,
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
        let rt = if rt_device { create_rt(&device, &base_entries) } else { None };
        let mut info = info;
        info.rt_cores = rt.is_some();
        match &rt {
            Some(_) => tracing::info!("RT core: ray query attive (tracciamento dei raggi in hardware)"),
            None if want_rt => tracing::info!("RT core non disponibili: tracciamento con la BVH dello shader"),
            None => tracing::info!("RT core spenti (FORGE_GPU_RT=0): tracciamento con la BVH dello shader"),
        }
        let mask = mask_buffer(&device, 1);
        let spectral_dummy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forge-gpu-spectral-dummy"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
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
            module,
            pipeline_layout: layout,
            spectral: None,
            spectral_dummy,
            rt,
        })
    }

    /// Vero se il tracciamento dei raggi usa gli RT core.
    pub fn uses_rt_cores(&self) -> bool {
        self.rt.is_some()
    }

    /// Pipeline e dati del render spettrale, creati la prima volta che servono.
    fn ensure_spectral(&mut self) {
        if self.spectral.is_some() {
            return;
        }
        let t0 = Instant::now();
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("forge-gpu-pipeline-spectral"),
            layout: Some(&self.pipeline_layout),
            module: &self.module,
            entry_point: Some("main_spectral"),
            compilation_options: Default::default(),
            cache: None,
        });
        use wgpu::util::DeviceExt;
        let data = forge_render_core::spectral::gpu_data();
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("forge-gpu-spectral"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        tracing::info!(
            "render spettrale sulla GPU pronto: {:.1} MB di dati in {:.0} ms",
            data.len() as f64 * 4.0 / 1e6,
            t0.elapsed().as_secs_f64() * 1e3
        );
        self.spectral = Some((pipeline, buffer));
    }

    /// Pipeline spettrale con le ray query, creata alla prima richiesta.
    fn ensure_spectral_rt(&mut self) {
        if let Some(rt) = &mut self.rt {
            if rt.spectral.is_none() {
                rt.spectral = Some(self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("forge-gpu-pipeline-spectral-rt"),
                    layout: Some(&rt.layout),
                    module: &rt.module,
                    entry_point: Some("main_spectral"),
                    compilation_options: Default::default(),
                    cache: None,
                }));
            }
        }
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
        let accel = if self.rt.is_some() {
            build_accel(&self.device, &self.queue, scene)
        } else {
            None
        };
        self.scene = Some(SceneGpu {
            scene_id: scene.content_id,
            buffer,
            params: sb.params,
            bytes,
            accel,
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
            "{} · {}{} · {} campioni/passata",
            self.info.name,
            self.info.backend,
            if self.rt.is_some() { " · RT core" } else { "" },
            self.samples_per_pass
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
        let spectral = settings.color_mode == forge_render_core::ColorMode::Spectral;
        if spectral {
            self.ensure_spectral();
        }
        // RT core se la scena ha le strutture di accelerazione hardware.
        let use_rt = self.rt.is_some() && self.scene.as_ref().is_some_and(|s| s.accel.is_some());
        if use_rt && spectral {
            self.ensure_spectral_rt();
        }
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
        fv(&mut p, P_ORTHO_H, if cam.lens.ortho { cam.ortho_half_height() } else { 0.0 });
        fv(&mut p, P_ORTHO_BACK, cam.ortho_back());
        fv(&mut p, P_LENS_R, cam.lens_radius());
        fv(&mut p, P_FOCUS, cam.lens.focus_distance.max(1e-4));
        p[P_BLADES] = cam.lens.blades;
        fv(&mut p, P_BLADE_ROT, cam.lens.blade_rotation_deg.to_radians());
        fv(&mut p, P_SHIFT_X, cam.lens.shift_x);
        fv(&mut p, P_SHIFT_Y, cam.lens.shift_y);
        self.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&p));

        let t = self.target.as_ref().expect("target");
        let rt = self.rt.as_ref().filter(|_| use_rt);
        let accel = sg.accel.as_ref().filter(|_| use_rt);
        let mut entries = vec![
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
            wgpu::BindGroupEntry {
                binding: 4,
                resource: match (&self.spectral, spectral) {
                    (Some((_, b)), true) => b.as_entire_binding(),
                    _ => self.spectral_dummy.as_entire_binding(),
                },
            },
        ];
        if let Some(a) = accel {
            entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: a.tlas.as_binding(),
            });
        }
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forge-gpu-bg"),
            layout: rt.map(|r| &r.bgl).unwrap_or(&self.bgl),
            entries: &entries,
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
            let pipeline = match (rt, spectral) {
                (Some(r), true) => r.spectral.as_ref().expect("pipeline spettrale RT"),
                (Some(r), false) => &r.main,
                (None, true) => &self.spectral.as_ref().expect("pipeline spettrale").0,
                (None, false) => &self.pipeline,
            };
            pass.set_pipeline(pipeline);
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
            rt_cores: false,
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
