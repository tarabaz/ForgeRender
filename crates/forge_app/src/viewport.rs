//! Viewport 3D rasterizzata con wgpu.
//!
//! Disegna la scena in una texture fuori schermo (MSAA 4×, depth buffer),
//! che viene poi mostrata dall'interfaccia egui come un'immagine. È una vista
//! di lavoro (shading semplice): il rendering fotorealistico è nella vista "Render".

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use forge_math::{Mat4, Vec3};
use forge_scene::{Camera, Scene};
use std::collections::BTreeSet;

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    selected: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 4],
    cam_pos: [f32; 4],
}

const SHADER: &str = r#"
struct U { view_proj: mat4x4<f32>, light_dir: vec4<f32>, cam_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) sel: f32,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) n: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) sel: f32,
    @location(3) wpos: vec3<f32>,
};

@vertex
fn vs(v: VIn) -> VOut {
    var o: VOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.n = v.normal;
    o.color = v.color;
    o.sel = v.sel;
    o.wpos = v.pos;
    return o;
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
    return vec4<f32>(c, 1.0);
}
"#;

struct Targets {
    size: [u32; 2],
    _color: wgpu::Texture,
    color_view: wgpu::TextureView,
    msaa_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
}

pub struct ViewportRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buf: Option<wgpu::Buffer>,
    index_buf: Option<wgpu::Buffer>,
    index_count: u32,
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
            label: Some("viewport-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attrs,
                })],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let info = rs.adapter.get_info();
        let adapter_info = format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type);
        tracing::info!("GPU viewport: {adapter_info}, driver {} {}", info.driver, info.driver_info);
        Self {
            pipeline,
            uniform_buf,
            bind_group,
            vertex_buf: None,
            index_buf: None,
            index_count: 0,
            targets: None,
            texture_id: None,
            adapter_info,
        }
    }

    /// Ricostruisce i buffer della geometria (componenti visibili, colori dei materiali, selezione).
    pub fn upload_scene(&mut self, rs: &egui_wgpu::RenderState, scene: &Scene, selection: &BTreeSet<usize>) {
        let mut verts: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (i, node) in scene.nodes.iter().enumerate() {
            let Some(mi) = node.mesh else { continue };
            if !scene.is_visible(i) {
                continue;
            }
            let selected = selection.iter().any(|&s| is_descendant_or_self(scene, i, s));
            let mesh = &scene.meshes[mi];
            let xf = scene.world_transform(i);
            let nxf = forge_math::Mat3::from_mat4(xf).inverse().transpose();
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
                let n = if has_n {
                    (nxf * mesh.normals[k]).normalize_or_zero()
                } else {
                    Vec3::Y
                };
                verts.push(Vertex {
                    pos: xf.transform_point3(*p).to_array(),
                    normal: n.to_array(),
                    color: color.to_array(),
                    selected: if selected { 1.0 } else { 0.0 },
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
        let color = mk(
            "viewport-color",
            COLOR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let msaa = mk("viewport-msaa", COLOR_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let depth = mk("viewport-depth", DEPTH_FORMAT, SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let color_view = color.create_view(&Default::default());
        let mut renderer = rs.renderer.write();
        match self.texture_id {
            Some(id) => renderer.update_egui_texture_from_wgpu_texture(device, &color_view, wgpu::FilterMode::Linear, id),
            None => self.texture_id = Some(renderer.register_native_texture(device, &color_view, wgpu::FilterMode::Linear)),
        }
        self.targets = Some(Targets {
            size,
            msaa_view: msaa.create_view(&Default::default()),
            depth_view: depth.create_view(&Default::default()),
            color_view,
            _color: color,
        });
    }

    /// Disegna la scena e restituisce la texture da mostrare.
    pub fn render(
        &mut self,
        rs: &egui_wgpu::RenderState,
        camera: &Camera,
        scene_radius: f32,
        size: [u32; 2],
        background: Vec3,
    ) -> Option<egui::TextureId> {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        self.ensure_targets(rs, size);
        let t = self.targets.as_ref()?;
        let aspect = size[0] as f32 / size[1] as f32;
        let dist = (camera.position - camera.target).length();
        let far = (dist + scene_radius * 4.0).max(1e-3);
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
        let mut enc = rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("viewport") });
        {
            let bg = background.to_array().map(|c| c as f64);
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa_view,
                    depth_slice: None,
                    resolve_target: Some(&t.color_view),
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
                    view: &t.depth_view,
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
            if let (Some(vb), Some(ib)) = (&self.vertex_buf, &self.index_buf) {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }
        rs.queue.submit(Some(enc.finish()));
        self.texture_id
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
