//! Viewport 3D rasterizzata con wgpu.
//!
//! Disegna la scena in una texture fuori schermo (MSAA 4×, depth buffer),
//! che viene poi mostrata dall'interfaccia egui come un'immagine. È una vista
//! di lavoro (shading semplice): il rendering fotorealistico è nella vista "Render".
//!
//! La geometria si carica una volta sola, nello spazio del suo oggetto; le matrici degli oggetti
//! stanno in un buffer a parte che il vertex shader legge. Muovere un oggetto (gizmo, timeline)
//! aggiorna solo quel buffer ([`ViewportRenderer::update_transforms`]): il costo non dipende dai
//! triangoli della scena.

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use forge_math::{Mat4, Vec3};
use forge_scene::{Camera, Scene};
use std::collections::BTreeSet;

/// Formato in cui si disegna: lo sfondo (colore lineare) e le superfici si scrivono codificati
/// sRGB, cioè come byte da mostrare.
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// Vista della stessa texture data a egui. egui 0.37 tratta le texture native come già in spazio
/// gamma (le sue sono `Rgba8Unorm`): leggendo la vista sRGB riceverebbe valori lineari e li
/// mostrerebbe come gamma, cioè molto più scuri (lo sfondo #353A42 diventava circa (9, 11, 14)).
/// Con la vista `Rgba8Unorm` egui legge i byte così come sono scritti.
const EGUI_VIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    selected: f32,
    /// Indice dell'oggetto nel buffer delle matrici.
    object: u32,
}

/// Matrici di un oggetto nel buffer letto dal vertex shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ObjectGpu {
    world: [[f32; 4]; 4],
    normal: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
    cam_pos: [f32; 4],
}

/// Griglia metrica: come appare (le misure sono nel mondo, in metri).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLook {
    /// Lato della cella in metri.
    pub cell_m: f32,
    /// Una linea principale ogni quante celle.
    pub major_every: u32,
    /// 1 = normale.
    pub intensity: f32,
}

impl GridLook {
    pub fn from_prefs(g: &crate::prefs::ViewportGrid) -> Option<Self> {
        g.show.then_some(Self {
            cell_m: g.cell_m,
            major_every: g.major_every,
            intensity: g.intensity,
        })
    }
}

/// Come si disegna la vista: sfondo (colore lineare), vista Mesh, griglia.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewStyle {
    pub background: Vec3,
    /// Spigoli dei triangoli sopra l'ombreggiatura.
    pub mesh_view: bool,
    pub grid: Option<GridLook>,
}

/// Parametri della griglia per lo shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GridUniforms {
    view_proj: [[f32; 4]; 4],
    /// Lato della cella, celle per linea principale, intensità, mezzo lato del piano disegnato.
    params: [f32; 4],
    /// Centro del piano (x, z) sul pavimento.
    center: [f32; 4],
}

/// Griglia sul pavimento (y = 0): un quadrato attorno al punto guardato, linee calcolate per
/// pixel nelle coordinate del mondo. Lo spessore è in pixel (fwidth), la spaziatura in metri:
/// lo zoom non cambia la misura delle celle. Quando un livello di linee diventa troppo fitto sullo
/// schermo sfuma e restano quelle principali (e le principali×N). Assi: X rosso, Z blu.
const GRID_SHADER: &str = r#"
struct G { view_proj: mat4x4<f32>, params: vec4<f32>, center: vec4<f32> };
@group(0) @binding(0) var<uniform> g: G;

struct GOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) xz: vec2<f32>,
};

@vertex
fn vs_grid(@builtin(vertex_index) i: u32) -> GOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let xz = g.center.xy + corners[i] * g.params.w;
    var o: GOut;
    o.clip = g.view_proj * vec4<f32>(xz.x, 0.0, xz.y, 1.0);
    o.xz = xz;
    return o;
}

// Linee ogni `size` metri: 1 sulla linea (circa un pixel), 0 tra le linee. Sfuma quando le
// celle sullo schermo scendono sotto i 5–16 pixel.
fn lines(xz: vec2<f32>, size: f32) -> f32 {
    let c = xz / size;
    let d = max(fwidth(c), vec2<f32>(1e-7));
    let px = abs(fract(c - 0.5) - 0.5) / d;
    let line = 1.0 - min(min(px.x, px.y), 1.0);
    let fade = 1.0 - smoothstep(1.0 / 16.0, 1.0 / 5.0, max(d.x, d.y));
    return line * fade;
}

fn axis(v: f32) -> f32 {
    return 1.0 - min(abs(v) / max(fwidth(v), 1e-7) / 1.5, 1.0);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

@fragment
fn fs_grid(i: GOut) -> @location(0) vec4<f32> {
    let cell = g.params.x;
    let every = g.params.y;
    let k = g.params.z;
    let minor = lines(i.xz, cell);
    let major = lines(i.xz, cell * every);
    let major2 = lines(i.xz, cell * every * every);
    var a = max(minor * 0.05, max(major * 0.13, major2 * 0.2));
    var col = vec3<f32>(0.85, 0.87, 0.9);
    let ax = axis(i.xz.y); // asse X: z = 0
    let az = axis(i.xz.x); // asse Z: x = 0
    if (ax > 0.0 && ax * 0.55 > a) {
        a = ax * 0.55;
        col = vec3<f32>(0.9, 0.3, 0.3);
    }
    if (az > 0.0 && az * 0.55 > a) {
        a = az * 0.55;
        col = vec3<f32>(0.3, 0.5, 0.95);
    }
    let r = length(i.xz - g.center.xy) / g.params.w;
    a = a * k * (1.0 - smoothstep(0.45, 1.0, r));
    return vec4<f32>(to_linear(col), clamp(a, 0.0, 1.0));
}
"#;

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, light_dir: vec4<f32>, cam_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;

struct Obj { world: mat4x4<f32>, normal: mat4x4<f32> };
@group(0) @binding(1) var<storage, read> objects: array<Obj>;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) sel: f32,
    @location(4) object: u32,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) n: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) sel: f32,
    @location(3) wpos: vec3<f32>,
};

fn shade_vertex(v: VIn) -> VOut {
    var o: VOut;
    let w = objects[v.object].world * vec4<f32>(v.pos, 1.0);
    o.clip = u.view_proj * w;
    o.n = (objects[v.object].normal * vec4<f32>(v.normal, 0.0)).xyz;
    o.color = v.color;
    o.sel = v.sel;
    o.wpos = w.xyz;
    return o;
}

@vertex
fn vs(v: VIn) -> VOut {
    return shade_vertex(v);
}

// Le superfici si sono sempre viste con il loro valore usato direttamente come byte (prima egui
// riceveva valori lineari e li mostrava come gamma): per non cambiarne l'aspetto ora che la
// texture arriva a egui con i byte giusti, il colore si passa in lineare.
fn shown(c: vec3<f32>) -> vec4<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let lo = x / 12.92;
    let hi = pow((x + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return vec4<f32>(select(hi, lo, x <= vec3<f32>(0.04045)), 1.0);
}

@fragment
fn fs(i: VOut) -> @location(0) vec4<f32> {
    var n = normalize(i.n);
    let v = normalize(u.cam_pos.xyz - i.wpos);
    if (dot(n, v) < 0.0) { n = -n; }
    let key = max(dot(n, u.light_dir.xyz), 0.0);
    let head = max(dot(n, v), 0.0);
    let hemi = mix(0.18, 0.4, n.y * 0.5 + 0.5);
    var c = i.color * (hemi + 0.5 * key + 0.3 * head);
    if (i.sel > 0.5) {
        let rim = pow(1.0 - head, 2.0);
        c = mix(c, vec3<f32>(1.0, 0.45, 0.05), 0.3) + vec3<f32>(1.0, 0.4, 0.0) * rim * 0.8;
    }
    return shown(c);
}

// Vista Mesh: spigoli dei triangoli, appena davanti alla superficie per non sparire nel depth.
@vertex
fn vs_wire(v: VIn) -> VOut {
    var o = shade_vertex(v);
    o.clip.z = max(o.clip.z - 3e-5 * o.clip.w - 1e-4 * abs(o.clip.z), 0.0);
    return o;
}

@fragment
fn fs_wire(i: VOut) -> @location(0) vec4<f32> {
    if (i.sel > 0.5) {
        return shown(vec3<f32>(1.0, 0.55, 0.15));
    }
    return shown(i.color * 0.25 + vec3<f32>(0.02, 0.02, 0.025));
}
"#;

struct Targets {
    size: [u32; 2],
    _color: wgpu::Texture,
    color_view: wgpu::TextureView,
    /// La stessa texture vista come `Rgba8Unorm`, per egui.
    _egui_view: wgpu::TextureView,
    msaa_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
}

pub struct ViewportRenderer {
    pipeline: wgpu::RenderPipeline,
    wire_pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    grid_pipeline: wgpu::RenderPipeline,
    grid_buf: wgpu::Buffer,
    grid_bind: wgpu::BindGroup,
    bind_layout: wgpu::BindGroupLayout,
    /// Legato al buffer delle matrici: si rifà con la geometria.
    bind_group: Option<wgpu::BindGroup>,
    object_buf: Option<wgpu::Buffer>,
    /// Nodi disegnati, nell'ordine del buffer delle matrici.
    objects: Vec<usize>,
    vertex_buf: Option<wgpu::Buffer>,
    index_buf: Option<wgpu::Buffer>,
    index_count: u32,
    /// Spigoli dei triangoli (vista Mesh), costruiti solo quando servono.
    edge_buf: Option<wgpu::Buffer>,
    edge_count: u32,
    /// Indici dei triangoli dell'ultimo upload, per costruire gli spigoli.
    tri_indices: Vec<u32>,
    targets: Option<Targets>,
    texture_id: Option<egui::TextureId>,
    pub adapter_info: String,
}

impl ViewportRenderer {
    pub fn new(rs: &egui_wgpu::RenderState) -> Self {
        let device = &rs.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32, 4 => Uint32];
        // Triangoli ombreggiati, oppure (vista Mesh) i loro spigoli come linee.
        let make = |label, vs, fs, topology, depth_write, compare| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attrs,
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    topology,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(compare),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = make(
            "viewport-pipeline",
            "vs",
            "fs",
            wgpu::PrimitiveTopology::TriangleList,
            true,
            wgpu::CompareFunction::Less,
        );
        let wire_pipeline = make(
            "viewport-wire",
            "vs_wire",
            "fs_wire",
            wgpu::PrimitiveTopology::LineList,
            false,
            wgpu::CompareFunction::LessEqual,
        );
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (grid_pipeline, grid_buf, grid_bind) = Self::grid_pipeline(device);
        let info = rs.adapter.get_info();
        let adapter_info = format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type);
        tracing::info!("GPU viewport: {adapter_info}, driver {} {}", info.driver, info.driver_info);
        Self {
            pipeline,
            wire_pipeline,
            uniform_buf,
            grid_pipeline,
            grid_buf,
            grid_bind,
            bind_layout: bgl,
            bind_group: None,
            object_buf: None,
            objects: Vec::new(),
            vertex_buf: None,
            index_buf: None,
            index_count: 0,
            edge_buf: None,
            edge_count: 0,
            tri_indices: Vec::new(),
            targets: None,
            texture_id: None,
            adapter_info,
        }
    }

    /// Griglia: piano trasparente sopra lo sfondo, dietro agli oggetti (prova la profondità,
    /// non la scrive), sullo stesso MSAA della scena.
    fn grid_pipeline(device: &wgpu::Device) -> (wgpu::RenderPipeline, wgpu::Buffer, wgpu::BindGroup) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport-grid"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport-grid-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport-grid-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport-grid"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_grid"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_grid"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport-grid-uniforms"),
            size: std::mem::size_of::<GridUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport-grid-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        });
        (pipeline, buf, bind)
    }

    /// Matrici dei nodi disegnati.
    fn object_data(&self, scene: &Scene) -> Vec<ObjectGpu> {
        self.objects
            .iter()
            .map(|&i| {
                let xf = if i < scene.nodes.len() {
                    scene.world_transform(i)
                } else {
                    Mat4::IDENTITY
                };
                let n = forge_math::Mat3::from_mat4(xf).inverse().transpose();
                ObjectGpu {
                    world: xf.to_cols_array_2d(),
                    normal: Mat4::from_mat3(n).to_cols_array_2d(),
                }
            })
            .collect()
    }

    /// Oggetti spostati, ruotati o scalati: aggiorna solo le loro matrici sulla GPU.
    pub fn update_transforms(&mut self, rs: &egui_wgpu::RenderState, scene: &Scene) {
        if let Some(buf) = &self.object_buf {
            rs.queue.write_buffer(buf, 0, bytemuck::cast_slice(&self.object_data(scene)));
        }
    }

    /// Ricostruisce i buffer della geometria (componenti visibili, colori dei materiali, selezione).
    pub fn upload_scene(&mut self, rs: &egui_wgpu::RenderState, scene: &Scene, selection: &BTreeSet<usize>) {
        let mut verts: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        self.edge_buf = None;
        self.tri_indices.clear();
        self.objects.clear();
        for (i, node) in scene.nodes.iter().enumerate() {
            let Some(mi) = node.mesh else { continue };
            if !scene.is_visible(i) {
                continue;
            }
            let selected = selection.iter().any(|&s| is_descendant_or_self(scene, i, s));
            let mesh = &scene.meshes[mi];
            let object = self.objects.len() as u32;
            self.objects.push(i);
            let color = scene
                .materials
                .get(scene.resolve_material(i))
                .map(|m| {
                    let c = Vec3::from_array(m.base_color);
                    if m.is_emissive() {
                        Vec3::ONE
                    } else if m.transmission > 0.5 {
                        Vec3::new(0.65, 0.8, 0.9)
                    } else {
                        c
                    }
                })
                .unwrap_or(Vec3::splat(0.8));
            let base = verts.len() as u32;
            let has_n = mesh.has_normals();
            for (k, p) in mesh.positions.iter().enumerate() {
                let n = if has_n { mesh.normals[k] } else { Vec3::Y };
                verts.push(Vertex {
                    pos: p.to_array(),
                    normal: n.to_array(),
                    color: color.to_array(),
                    selected: if selected { 1.0 } else { 0.0 },
                    object,
                });
            }
            indices.extend(mesh.indices.iter().flat_map(|t| t.map(|v| v + base)));
        }
        let device = &rs.device;
        let limit = device.limits().max_buffer_size;
        let vbytes = (verts.len() * std::mem::size_of::<Vertex>()) as u64;
        if vbytes > limit || (indices.len() * 4) as u64 > limit {
            tracing::warn!(
                "geometria troppo grande per la viewport ({} MB): non visualizzata",
                vbytes / 1_000_000
            );
            self.index_count = 0;
            return;
        }
        use wgpu::util::DeviceExt;
        self.index_count = indices.len() as u32;
        if indices.is_empty() {
            self.vertex_buf = None;
            self.index_buf = None;
            return;
        }
        self.vertex_buf = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport-vertices"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        self.index_buf = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport-indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }));
        self.tri_indices = indices;
        let objects = self.object_data(scene);
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport-objects"),
            contents: bytemuck::cast_slice(&objects),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport-bg"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buf.as_entire_binding(),
                },
            ],
        }));
        self.object_buf = Some(buf);
    }

    /// Spigoli dei triangoli per la vista Mesh (dall'ultimo upload; una volta sola).
    fn ensure_edges(&mut self, rs: &egui_wgpu::RenderState) {
        if self.edge_buf.is_some() || self.tri_indices.is_empty() {
            return;
        }
        let edges: Vec<u32> = self
            .tri_indices
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|t| [t[0], t[1], t[1], t[2], t[2], t[0]])
            .collect();
        if (edges.len() * 4) as u64 > rs.device.limits().max_buffer_size {
            tracing::warn!("troppi triangoli per la vista Mesh: spigoli non mostrati");
            self.tri_indices.clear();
            return;
        }
        use wgpu::util::DeviceExt;
        self.edge_count = edges.len() as u32;
        self.edge_buf = Some(rs.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport-edges"),
            contents: bytemuck::cast_slice(&edges),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }

    fn ensure_targets(&mut self, rs: &egui_wgpu::RenderState, size: [u32; 2]) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let device = &rs.device;
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let mk = |label, format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport-color"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[EGUI_VIEW_FORMAT],
        });
        let msaa = mk("viewport-msaa", COLOR_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let depth = mk("viewport-depth", DEPTH_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let color_view = color.create_view(&Default::default());
        let egui_view = color.create_view(&wgpu::TextureViewDescriptor {
            format: Some(EGUI_VIEW_FORMAT),
            ..Default::default()
        });
        let mut renderer = rs.renderer.write();
        match self.texture_id {
            Some(id) => renderer.update_egui_texture_from_wgpu_texture(device, &egui_view, wgpu::FilterMode::Linear, id),
            None => self.texture_id = Some(renderer.register_native_texture(device, &egui_view, wgpu::FilterMode::Linear)),
        }
        self.targets = Some(Targets {
            size,
            msaa_view: msaa.create_view(&Default::default()),
            depth_view: depth.create_view(&Default::default()),
            color_view,
            _egui_view: egui_view,
            _color: color,
        });
    }

    /// Un fotogramma come quello della viewport su texture proprie, riletto dalla GPU (RGBA a 8
    /// bit, sRGB, riga per riga): i fotogrammi dell'anteprima dell'animazione.
    pub fn capture(
        &mut self,
        rs: &egui_wgpu::RenderState,
        camera: &Camera,
        scene_radius: f32,
        size: [u32; 2],
        background: Vec3,
        mesh_view: bool,
    ) -> Option<Vec<u8>> {
        let device = &rs.device;
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let mk = |format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("viewport-cattura"),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = mk(
            COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let msaa = mk(COLOR_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let depth = mk(DEPTH_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let (cv, mv, dv) = (
            color.create_view(&Default::default()),
            msaa.create_view(&Default::default()),
            depth.create_view(&Default::default()),
        );
        if mesh_view {
            self.ensure_edges(rs);
        }
        self.write_uniforms(rs, camera, scene_radius, size, None);
        let mut enc = rs.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport-cattura"),
        });
        let style = ViewStyle {
            background,
            mesh_view,
            grid: None,
        };
        self.draw(&mut enc, &mv, &cv, &dv, &style);
        rs.queue.submit(Some(enc.finish()));
        Self::read_rgba(rs, &color, size)
    }

    /// I byte che egui mostra dell'ultimo fotogramma di [`Self::render`] (la vista
    /// `Rgba8Unorm` legge esattamente questi).
    #[cfg(test)]
    fn shown_bytes(&self, rs: &egui_wgpu::RenderState) -> Option<Vec<u8>> {
        let t = self.targets.as_ref()?;
        Self::read_rgba(rs, &t._color, t.size)
    }

    /// Rilegge dalla GPU una texture RGBA a 8 bit, riga per riga.
    fn read_rgba(rs: &egui_wgpu::RenderState, color: &wgpu::Texture, size: [u32; 2]) -> Option<Vec<u8>> {
        let device = &rs.device;
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport-lettura"),
        });
        // Le righe nel buffer vanno allineate.
        let row = size[0] * 4;
        let aligned = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport-lettura"),
            size: aligned as u64 * size[1] as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        enc.copy_texture_to_buffer(
            color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned),
                    rows_per_image: Some(size[1]),
                },
            },
            extent,
        );
        rs.queue.submit(Some(enc.finish()));
        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().ok()?.ok()?;
        let data = slice.get_mapped_range().ok()?;
        let mut out = Vec::with_capacity((row * size[1]) as usize);
        for y in 0..size[1] as usize {
            let start = y * aligned as usize;
            out.extend_from_slice(&data[start..start + row as usize]);
        }
        drop(data);
        readback.unmap();
        Some(out)
    }

    /// Disegna la scena e restituisce la texture da mostrare.
    pub fn render(
        &mut self,
        rs: &egui_wgpu::RenderState,
        camera: &Camera,
        scene_radius: f32,
        size: [u32; 2],
        style: ViewStyle,
    ) -> Option<egui::TextureId> {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        self.ensure_targets(rs, size);
        if style.mesh_view {
            self.ensure_edges(rs);
        }
        self.write_uniforms(rs, camera, scene_radius, size, style.grid);
        let mut enc = rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("viewport") });
        let t = self.targets.as_ref()?;
        self.draw(&mut enc, &t.msaa_view, &t.color_view, &t.depth_view, &style);
        rs.queue.submit(Some(enc.finish()));
        self.texture_id
    }

    /// Camera e luce per il fotogramma (e la griglia, se c'è).
    fn write_uniforms(&self, rs: &egui_wgpu::RenderState, camera: &Camera, scene_radius: f32, size: [u32; 2], grid: Option<GridLook>) {
        let aspect = size[0] as f32 / size[1] as f32;
        let dist = (camera.position - camera.target).length();
        // La griglia si stende ben oltre la scena (sfuma verso il bordo): il piano lontano la
        // deve contenere.
        let grid_extent = grid.map(|_| {
            let span = if camera.lens.ortho {
                camera.ortho_half_height() * aspect.max(1.0)
            } else {
                dist
            };
            (span * 12.0).max(scene_radius * 4.0).max(1e-3)
        });
        let far = (dist + scene_radius * 4.0 + grid_extent.map_or(0.0, |e| e * 1.1)).max(1e-3);
        let near = (far * 1e-4).max(1e-5);
        let vp: Mat4 = camera.projection_matrix(aspect, near, far) * camera.view_matrix();
        let (r, u, f) = camera.basis();
        let light = (-f + u * 0.8 - r * 0.5).normalize();
        let uni = Uniforms {
            view_proj: vp.to_cols_array_2d(),
            light_dir: light.extend(0.0).to_array(),
            cam_pos: camera.position.extend(1.0).to_array(),
        };
        rs.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uni));
        if let (Some(gl), Some(extent)) = (grid, grid_extent) {
            let gu = GridUniforms {
                view_proj: vp.to_cols_array_2d(),
                params: [gl.cell_m.max(1e-5), gl.major_every.max(2) as f32, gl.intensity, extent],
                center: [camera.target.x, camera.target.z, 0.0, 0.0],
            };
            rs.queue.write_buffer(&self.grid_buf, 0, bytemuck::bytes_of(&gu));
        }
    }

    /// Il passaggio di disegno sulle texture indicate.
    fn draw(
        &self,
        enc: &mut wgpu::CommandEncoder,
        msaa: &wgpu::TextureView,
        resolve: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        style: &ViewStyle,
    ) {
        let (mesh_view, grid) = (style.mesh_view, style.grid.is_some());
        {
            let bg = style.background.to_array().map(|c| c as f64);
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa,
                    depth_slice: None,
                    resolve_target: Some(resolve),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: bg[0],
                            g: bg[1],
                            b: bg[2],
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let (Some(vb), Some(ib), Some(bg)) = (&self.vertex_buf, &self.index_buf, &self.bind_group) {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, bg, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
                if let (true, Some(eb)) = (mesh_view, &self.edge_buf) {
                    pass.set_pipeline(&self.wire_pipeline);
                    pass.set_index_buffer(eb.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..self.edge_count, 0, 0..1);
                }
            }
            // Dopo gli oggetti: li rispetta con la profondità.
            if grid {
                pass.set_pipeline(&self.grid_pipeline);
                pass.set_bind_group(0, &self.grid_bind, &[]);
                pass.draw(0..6, 0..1);
            }
        }
    }
}

fn is_descendant_or_self(scene: &Scene, mut node: usize, ancestor: usize) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        match scene.nodes.get(node).and_then(|n| n.parent) {
            Some(p) => node = p,
            None => return false,
        }
    }
}

/// Proietta un punto 3D sullo schermo (rettangolo della viewport). `None` se dietro la camera.
pub fn project(camera: &Camera, rect: egui::Rect, p: Vec3) -> Option<egui::Pos2> {
    let aspect = rect.width() / rect.height().max(1.0);
    let vp = camera.projection_matrix(aspect, 1e-4, 1e6) * camera.view_matrix();
    let c = vp * p.extend(1.0);
    if c.w <= 1e-6 {
        return None;
    }
    let ndc = c.truncate() / c.w;
    Some(egui::pos2(
        rect.left() + (ndc.x * 0.5 + 0.5) * rect.width(),
        rect.top() + (0.5 - ndc.y * 0.5) * rect.height(),
    ))
}
/// I test di questo programma che usano la GPU passano uno alla volta: con più dispositivi
/// creati insieme la scheda può negare Vulkan a uno di loro, che ripiega su DirectX 12.
#[cfg(test)]
pub(crate) fn gpu_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GPU_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    GPU_TESTS.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GPU senza finestra, con il renderer di egui come quello del programma (uscita gamma).
    fn headless() -> Option<egui_wgpu::RenderState> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        let renderer = egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, Default::default());
        Some(egui_wgpu::RenderState {
            adapter,
            available_adapters: Vec::new(),
            instance,
            device,
            queue,
            target_format: wgpu::TextureFormat::Rgba8Unorm,
            renderer: std::sync::Arc::new(egui::mutex::RwLock::new(renderer)),
        })
    }

    fn pixel(bytes: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * w + x) * 4) as usize;
        [bytes[i], bytes[i + 1], bytes[i + 2]]
    }

    fn close(a: [u8; 3], b: [u8; 3]) -> bool {
        a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) <= 1)
    }

    /// Lo sfondo che egui mostra ha esattamente i byte del colore scelto (i quattro preset e un
    /// personalizzato), non il suo valore lineare (#353A42 usciva circa (9, 11, 14)). Il
    /// fotogramma si rilegge dalla texture che egui legge con la vista `Rgba8Unorm`.
    #[test]
    fn background_bytes_match_the_chosen_colour() {
        let _gpu = gpu_test_guard();
        let Some(rs) = headless() else {
            eprintln!("nessuna GPU: test saltato");
            return;
        };
        let mut vp = ViewportRenderer::new(&rs);
        let mut colours: Vec<[u8; 3]> = crate::prefs::VIEWPORT_BACKGROUNDS.iter().map(|p| p.2).collect();
        colours.push([200, 10, 77]);
        for c in colours {
            let bg = Vec3::from_array(crate::ui::color_picker::from_srgb8(c));
            let style = ViewStyle {
                background: bg,
                mesh_view: false,
                grid: None,
            };
            vp.render(&rs, &Camera::default(), 1.0, [64, 48], style).unwrap();
            let bytes = vp.shown_bytes(&rs).unwrap();
            for (x, y) in [(0, 0), (32, 24), (63, 47)] {
                let got = pixel(&bytes, 64, x, y);
                assert!(close(got, c), "atteso {c:?}, egui riceve {got:?}");
            }
        }
        // La vista per egui è senza conversione sRGB: i byte sono quelli mostrati.
        assert_eq!(EGUI_VIEW_FORMAT, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(COLOR_FORMAT.remove_srgb_suffix(), EGUI_VIEW_FORMAT);
    }

    /// Griglia vista dall'alto in ortografica, 1 pixel = 1 mm: le linee cadono ogni 2 cm nel
    /// mondo, tra le linee resta lo sfondo; spenta non disegna niente; il fotogramma per
    /// l'animazione (capture) non la contiene.
    #[test]
    fn grid_lines_are_at_world_distances() {
        let _gpu = gpu_test_guard();
        let Some(rs) = headless() else {
            eprintln!("nessuna GPU: test saltato");
            return;
        };
        let mut vp = ViewportRenderer::new(&rs);
        let mut cam = Camera {
            position: Vec3::new(0.0, 1.0, 0.0),
            target: Vec3::ZERO,
            up: Vec3::new(0.0, 0.0, -1.0),
            ..Default::default()
        };
        cam.lens.ortho = true;
        cam.lens.ortho_height = 0.2;
        let c = [40, 44, 52];
        let bg = Vec3::from_array(crate::ui::color_picker::from_srgb8(c));
        let grid = GridLook {
            cell_m: 0.02,
            major_every: 5,
            intensity: 1.0,
        };
        let size = [200, 200];
        let style = ViewStyle {
            background: bg,
            mesh_view: false,
            grid: Some(grid),
        };
        vp.render(&rs, &cam, 0.1, size, style).unwrap();
        let on = vp.shown_bytes(&rs).unwrap();
        let lum = |p: [u8; 3]| p.iter().map(|&v| v as u32).sum::<u32>();
        // Pixel 100 = x 0; linee a x = ±2, ±4 cm → colonne 80, 120, 140 (bordo tra due pixel:
        // si guarda il più chiaro dei due). Righe lontane dalle linee orizzontali: 110 (z = 1 cm).
        for col in [80u32, 120, 140, 60] {
            let a = pixel(&on, 200, col, 110).max(pixel(&on, 200, col - 1, 110));
            assert!(lum(a) > lum(c) + 6, "colonna {col}: linea attesa, trovato {a:?}");
        }
        for col in [90u32, 110, 130, 150] {
            let a = pixel(&on, 200, col, 110);
            assert!(close(a, c), "colonna {col}: tra le linee lo sfondo, trovato {a:?}");
        }
        // Colonna 100: asse Z (x = 0), blu.
        let axis = pixel(&on, 200, 100, 110).max(pixel(&on, 200, 99, 110));
        assert!(axis[2] > axis[0] + 10, "asse Z blu, trovato {axis:?}");
        // Spenta: solo sfondo.
        vp.render(&rs, &cam, 0.1, size, ViewStyle { grid: None, ..style }).unwrap();
        let off = vp.shown_bytes(&rs).unwrap();
        assert!(off.chunks(4).all(|p| close([p[0], p[1], p[2]], c)), "griglia spenta: solo sfondo");
        // I fotogrammi dell'anteprima dell'animazione non hanno la griglia.
        let cap = vp.capture(&rs, &cam, 0.1, size, bg, false).unwrap();
        assert!(cap.chunks(4).all(|p| close([p[0], p[1], p[2]], c)));
    }
}
