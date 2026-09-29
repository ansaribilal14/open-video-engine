//! GPU reference executor (wave 9 — E-005 promotion, ADR-020).
//!
//! Executes the SAME RenderPlan the software executor executes, on wgpu,
//! with the SAME integer arithmetic — byte parity with the software path is
//! the acceptance gate (BUILD_PLAN wave 6: "golden parity software↔GPU").
//! Software stays the correctness reference; this backend never replaces it,
//! it must MATCH it (directive §11 software-first).
//!
//! Byte-exactness contract (mirrors exec.rs arithmetic 1:1):
//!   * all textures are Rgba8Uint — NO float normalization anywhere
//!     (`Rgba8Unorm` would quantize through float encoding and break
//!     exactness; E-005 used uint loads for the same reason);
//!   * compositing math in the shader is the identical u32 pipeline:
//!     `d = 255·den`, `n_s = min(sa·a_num, d)`, `inv = d − n_s`,
//!     `out = (cs·n_s + cd·inv + d/2) / d`, output alpha forced 255;
//!   * fragment coordinates come from `@builtin(position)` — NEVER
//!     interpolated varyings (E-005 finding #2: varyings shifted pixels);
//!   * geometry is the same integer translation (bounds-clipped, transparent
//!     outside) as exec.rs Transform;
//!   * ColorConvert stays a tag stamp for the RGBA v1 path (no pixel work).
//!
//! Overflow-bounded denominators: u32 arithmetic bounds the blend to
//! reduced `den ≤ 65_000` (worst case 255·d + d/2 = 65 152.5·den must stay
//! under 2³² ⇒ den ≤ 65 927; 65 000 is the enforced margin). Beyond it the
//! executor returns a typed error and the CALLER falls back to software —
//! never a silent divergence.
//!
//! Feature-detect + fallback (BUILD_PLAN wave 6): [`GpuRenderer::new`]
//! returns a typed [`GpuError::NoAdapter`] when no device is available;
//! nothing panics, nothing silently degrades — the caller chooses software.

use std::collections::HashMap;
use std::sync::mpsc;

use wgpu::util::DeviceExt;

use crate::exec::FrameSource;
use crate::plan::{PassKind, RenderPlan, SourceId, SurfaceId};
use ove_media::{BackendId, BitDepth, ColorTags, FrameBytes, FrameEnvelope, PixelFormat, StreamId};
use ove_time::Rational;

/// Largest reduced alpha denominator accepted on the GPU path (u32 bound —
/// see module docs). Larger values are a typed error, not a clamp.
pub const MAX_ALPHA_DEN: i64 = 65_000;
/// Largest per-dimension texture the default device limits admit with margin.
pub const MAX_TEXTURE_DIM: u32 = 8192;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuError {
    /// No usable adapter (feature-detect failed) — fall back to software.
    NoAdapter { detail: String },
    /// Adapter present but device acquisition or GPU I/O failed.
    Device { detail: String },
    /// A plan parameter the u32 shader pipeline cannot execute exactly.
    UnsupportedAlphaDen { den: i64, max: i64 },
    /// Frame geometry beyond the device texture limit.
    FrameTooLarge { width: u32, height: u32, max: u32 },
    /// Pass-level failure with software-identical semantics.
    Render(crate::exec::RenderError),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::NoAdapter { detail } => write!(f, "no GPU adapter available: {detail}"),
            GpuError::Device { detail } => write!(f, "GPU device failure: {detail}"),
            GpuError::UnsupportedAlphaDen { den, max } => write!(
                f,
                "blend alpha denominator {den} exceeds the GPU u32 bound {max} — use the software executor"
            ),
            GpuError::FrameTooLarge {
                width,
                height,
                max,
            } => write!(f, "frame {width}×{height} exceeds the GPU texture limit {max}"),
            GpuError::Render(e) => write!(f, "GPU pass execution: {e}"),
        }
    }
}

impl std::error::Error for GpuError {}

impl From<crate::exec::RenderError> for GpuError {
    fn from(e: crate::exec::RenderError) -> Self {
        GpuError::Render(e)
    }
}

const SHADER_XFORM: &str = r#"
struct Xform {
    dx: i32,
    dy: i32,
    _pad0: u32,
    _pad1: u32,
};
@group(0) @binding(0) var src_tex: texture_2d<u32>;
@group(0) @binding(1) var<uniform> x: Xform;

struct VsOut {
    @builtin(position) pos: vec4f,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    // fullscreen triangle; every target pixel is written (no clear needed)
    var p = array(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    var out: VsOut;
    out.pos = vec4f(p[vi], 0.0, 1.0);
    return out;
}

@fragment
fn fs_transform(in: VsOut) -> @location(0) vec4<u32> {
    // E-005 rule: framebuffer coords from @builtin(position) — never varyings
    let px = vec2i(i32(in.pos.x), i32(in.pos.y));
    let dims = textureDimensions(src_tex);
    let sx = px.x - x.dx;
    let sy = px.y - x.dy;
    if (sx < 0 || sy < 0 || sx >= i32(dims.x) || sy >= i32(dims.y)) {
        return vec4<u32>(0u, 0u, 0u, 0u);
    }
    return textureLoad(src_tex, vec2<u32>(u32(sx), u32(sy)), 0);
}
"#;

const SHADER_BLEND: &str = r#"
struct Blend {
    a_num: u32,
    den: u32,
    src_dx: i32,
    src_dy: i32,
};
@group(0) @binding(0) var src_tex: texture_2d<u32>;
@group(0) @binding(1) var dst_tex: texture_2d<u32>;
@group(0) @binding(2) var<uniform> b: Blend;

struct VsOut {
    @builtin(position) pos: vec4f,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var p = array(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    var out: VsOut;
    out.pos = vec4f(p[vi], 0.0, 1.0);
    return out;
}

fn sample_shifted(px: vec2i, dx: i32, dy: i32) -> vec4<u32> {
    let dims = textureDimensions(src_tex);
    let sx = px.x - dx;
    let sy = px.y - dy;
    if (sx < 0 || sy < 0 || sx >= i32(dims.x) || sy >= i32(dims.y)) {
        return vec4<u32>(0u, 0u, 0u, 0u);
    }
    return textureLoad(src_tex, vec2<u32>(u32(sx), u32(sy)), 0);
}

@fragment
fn fs_blend(in: VsOut) -> @location(0) vec4<u32> {
    let px = vec2i(i32(in.pos.x), i32(in.pos.y));
    let s = sample_shifted(px, b.src_dx, b.src_dy);
    let d_px = textureLoad(dst_tex, vec2<u32>(u32(px.x), u32(px.y)), 0);
    // identical integer arithmetic to exec.rs BlendOver
    let d = b.den;
    let n_s = min(s.a * b.a_num, d);
    let inv = d - n_s;
    var out = vec4<u32>(255u, 255u, 255u, 255u);
    out.r = (s.r * n_s + d_px.r * inv + d / 2u) / d;
    out.g = (s.g * n_s + d_px.g * inv + d / 2u) / d;
    out.b = (s.b * n_s + d_px.b * inv + d / 2u) / d;
    return out;
}
"#;

/// One live GPU surface: texture + color tags. Sampling rules (the exact
/// analog of the software executor's normalized surfaces):
///   * a FETCH surface is the native frame; sampling at (x,y) with native
///     bounds-check reproduces software's transparent-padded paste-at-origin;
///   * a TRANSFORM/blend target is output-sized with content already at its
///     final output coordinates — later passes sample it directly (no extra
///     offset; the transform's own dx/dy did the placement).
struct GpuSurface {
    tex: wgpu::Texture,
    tags: ColorTags,
}

/// The wgpu executor. Construct once, execute many frames.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline_transform: wgpu::RenderPipeline,
    pipeline_blend: wgpu::RenderPipeline,
    adapter_summary: String,
}

impl GpuRenderer {
    /// Feature-detect + device acquisition. Typed `NoAdapter` when the host
    /// has no usable device — the documented trigger for software fallback.
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| GpuError::NoAdapter {
            detail: e.to_string(),
        })?;
        let info = adapter.get_info();
        let adapter_summary = format!("{} / {} / {:?}", info.name, info.driver, info.device_type);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ove-render-gpu"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: Default::default(),
        }))
        .map_err(|e| GpuError::Device {
            detail: e.to_string(),
        })?;

        let targets_uint = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Uint,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let mk = |shader: &str, frag: &str, entries: &[wgpu::BindGroupLayoutEntry], label: &str| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(shader.into()),
            });
            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(frag),
                    compilation_options: Default::default(),
                    targets: &targets_uint,
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };

        let tex_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Uint,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let uniform_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let pipeline_transform = mk(
            SHADER_XFORM,
            "fs_transform",
            &[tex_entry(0), uniform_entry(1)],
            "ove-xform",
        );
        let pipeline_blend = mk(
            SHADER_BLEND,
            "fs_blend",
            &[tex_entry(0), tex_entry(1), uniform_entry(2)],
            "ove-blend",
        );

        Ok(GpuRenderer {
            device,
            queue,
            pipeline_transform,
            pipeline_blend,
            adapter_summary,
        })
    }

    /// Adapter identity for logs / honest perf classification.
    pub fn adapter_summary(&self) -> &str {
        &self.adapter_summary
    }

    fn output_sized_texture(&self, w: u32, h: u32, label: &str) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    fn fullscreen_pass(
        &self,
        pipeline: &wgpu::RenderPipeline,
        bg: &wgpu::BindGroup,
        target_view: &wgpu::TextureView,
        label: &str,
    ) {
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations::default(), // Load: every pixel is written
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, bg, &[]);
            rp.draw(0..3, 0..1);
        }
        self.queue.submit([enc.finish()]);
    }

    /// Execute one compiled plan → one output FrameEnvelope, byte-identical
    /// to the software executor's on every input the u32 bound admits.
    pub fn execute_frame(
        &self,
        plan: &RenderPlan,
        sources: &HashMap<SourceId, &dyn FrameSource>,
    ) -> Result<FrameEnvelope, GpuError> {
        let spec = &plan.output;
        if spec.width > MAX_TEXTURE_DIM || spec.height > MAX_TEXTURE_DIM {
            return Err(GpuError::FrameTooLarge {
                width: spec.width,
                height: spec.height,
                max: MAX_TEXTURE_DIM,
            });
        }
        // Pre-validate every blend alpha (typed loud failure BEFORE any GPU
        // work; the plan validator owns [0,1], we own the u32 bound).
        for p in &plan.passes {
            if let PassKind::BlendOver { alpha } = p.kind {
                let den = reduced_den(alpha);
                if den > MAX_ALPHA_DEN {
                    return Err(GpuError::UnsupportedAlphaDen {
                        den,
                        max: MAX_ALPHA_DEN,
                    });
                }
            }
        }

        let mut scratches: HashMap<SurfaceId, GpuSurface> = HashMap::new();
        // OPAQUE black base — the exact software base (pre-filled via the
        // copy path; every following pass fully overwrites its own target).
        let mut output = self.output_sized_texture(spec.width, spec.height, "ove-output");
        {
            let px = (spec.width as usize) * (spec.height as usize);
            let mut black = Vec::with_capacity(px * 4);
            for _ in 0..px {
                black.extend_from_slice(&[0, 0, 0, 255]);
            }
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &output,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &black,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(spec.width * 4),
                    rows_per_image: Some(spec.height),
                },
                wgpu::Extent3d {
                    width: spec.width,
                    height: spec.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        for pass in &plan.passes {
            match pass.kind {
                PassKind::SourceFetch { source, src_pts } => {
                    let src = sources.get(&source).ok_or({
                        GpuError::Render(crate::exec::RenderError::SourceFrameMissing {
                            source,
                            at: src_pts,
                        })
                    })?;
                    let env = src.fetch(src_pts).ok_or(GpuError::Render(
                        crate::exec::RenderError::SourceFrameMissing {
                            source,
                            at: src_pts,
                        },
                    ))?;
                    let bytes = env.cpu_bytes().ok_or(GpuError::Render(
                        crate::exec::RenderError::FrameNotCpuRgba { source },
                    ))?;
                    if env.pixel_format != PixelFormat::Rgba || env.bit_depth != BitDepth::B8 {
                        return Err(GpuError::Render(
                            crate::exec::RenderError::FrameNotCpuRgba { source },
                        ));
                    }
                    // single-conversion rule — same loud check as software
                    if env.color != spec.working_space {
                        let has_convert = plan.passes.iter().any(|p| {
                            p.clip_id == pass.clip_id
                                && matches!(p.kind, PassKind::ColorConvert { .. })
                        });
                        if !has_convert {
                            return Err(GpuError::Render(crate::exec::RenderError::TagMismatch {
                                source,
                                frame: env.color,
                                expected: spec.working_space,
                            }));
                        }
                    }
                    let (sw, sh) = (env.width, env.height);
                    if sw > MAX_TEXTURE_DIM || sh > MAX_TEXTURE_DIM {
                        return Err(GpuError::FrameTooLarge {
                            width: sw,
                            height: sh,
                            max: MAX_TEXTURE_DIM,
                        });
                    }
                    let stride = bytes.strides.first().copied().unwrap_or((sw as usize) * 4);
                    let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("ove-fetch"),
                        size: wgpu::Extent3d {
                            width: sw,
                            height: sh,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Uint,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &bytes.data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(stride as u32),
                            rows_per_image: Some(sh),
                        },
                        wgpu::Extent3d {
                            width: sw,
                            height: sh,
                            depth_or_array_layers: 1,
                        },
                    );
                    scratches.insert(
                        s_of(pass.outputs.first().copied()),
                        GpuSurface {
                            tex,
                            tags: env.color,
                        },
                    );
                }
                PassKind::ColorConvert { to } => {
                    // tag stamp (RGBA v1) — identical software semantics
                    let in_id = s_of(pass.inputs.first().copied());
                    let out_id = s_of(pass.outputs.first().copied());
                    let mut s = scratches.remove(&in_id).ok_or(GpuError::Render(
                        crate::exec::RenderError::MissingSurface(in_id),
                    ))?;
                    s.tags = to;
                    scratches.insert(out_id, s);
                }
                PassKind::Transform { dx, dy } => {
                    let in_id = s_of(pass.inputs.first().copied());
                    let input = scratches.remove(&in_id).ok_or(GpuError::Render(
                        crate::exec::RenderError::MissingSurface(in_id),
                    ))?;
                    let target =
                        self.output_sized_texture(spec.width, spec.height, "ove-xform-target");
                    let target_view = target.create_view(&Default::default());
                    let src_view = input.tex.create_view(&Default::default());
                    let ubuf = self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("ove-xform-uniform"),
                            contents: &[
                                dx.to_le_bytes(),
                                dy.to_le_bytes(),
                                0u32.to_le_bytes(),
                                0u32.to_le_bytes(),
                            ]
                            .concat(),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                    let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("ove-xform-bg"),
                        layout: &self.pipeline_transform.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&src_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: ubuf.as_entire_binding(),
                            },
                        ],
                    });
                    self.fullscreen_pass(
                        &self.pipeline_transform,
                        &bg,
                        &target_view,
                        "ove-xform-pass",
                    );
                    let tags = input.tags;
                    scratches.insert(
                        s_of(pass.outputs.first().copied()),
                        GpuSurface { tex: target, tags },
                    );
                }
                PassKind::BlendOver { alpha } => {
                    let in_id = s_of(pass.inputs.first().copied());
                    let input = scratches.remove(&in_id).ok_or(GpuError::Render(
                        crate::exec::RenderError::MissingSurface(in_id),
                    ))?;
                    let target =
                        self.output_sized_texture(spec.width, spec.height, "ove-blend-target");
                    let target_view = target.create_view(&Default::default());
                    let src_view = input.tex.create_view(&Default::default());
                    let out_view = output.create_view(&Default::default());
                    let a_num = alpha.num().max(0) as u32;
                    // shader uniform carries d = 255·den (the software
                    // executor's divisor), NOT the bare denominator
                    let d = 255u32 * (reduced_den(alpha) as u32);
                    // the input surface already carries content at its final
                    // output coordinates (see GpuSurface docs) — sample it
                    // directly, exactly like the software blend's 1:1 zip
                    let (sdx, sdy) = (0i32, 0i32);
                    let ubuf = self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("ove-blend-uniform"),
                            contents: &[
                                a_num.to_le_bytes(),
                                d.to_le_bytes(),
                                sdx.to_le_bytes(),
                                sdy.to_le_bytes(),
                            ]
                            .concat(),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                    let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("ove-blend-bg"),
                        layout: &self.pipeline_blend.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&src_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&out_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: ubuf.as_entire_binding(),
                            },
                        ],
                    });
                    self.fullscreen_pass(&self.pipeline_blend, &bg, &target_view, "ove-blend-pass");
                    // the composited result becomes the new output surface
                    output = target;
                }
            }
        }

        // ---- readback: padded rows → strip to w*4 tight bytes ----
        let bytes_per_row = (spec.width * 4).div_ceil(256) * 256;
        let buf_size = (bytes_per_row as u64) * (spec.height as u64);
        let rbuf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ove-readback"),
            size: buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ove-readback-enc"),
            });
        enc.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &rbuf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(spec.height),
                },
            },
            wgpu::Extent3d {
                width: spec.width,
                height: spec.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([enc.finish()]);
        let slice = rbuf.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::Wait);
        rx.recv()
            .map_err(|_| GpuError::Device {
                detail: "readback channel closed".into(),
            })?
            .map_err(|e| GpuError::Device {
                detail: format!("map_async failed: {e}"),
            })?;
        let padded = slice.get_mapped_range();
        let w4 = (spec.width as usize) * 4;
        let mut data = Vec::with_capacity(w4 * spec.height as usize);
        for y in 0..spec.height as usize {
            let from = y * bytes_per_row as usize;
            data.extend_from_slice(&padded[from..from + w4]);
        }
        drop(padded);
        rbuf.unmap();

        let pts = spec.frame_pts(plan.frame_index);
        let duration = Rational::new(spec.rate_den, spec.rate_num);
        let mut env = FrameEnvelope::video_cpu(
            pts,
            duration,
            StreamId(0),
            spec.width,
            spec.height,
            PixelFormat::Rgba,
            BitDepth::B8,
            spec.working_space,
            FrameBytes {
                data,
                strides: vec![(spec.width as usize) * 4],
            },
            plan.frame_index == 0,
            BackendId::Other(format!("ove-render/gpu ({})", self.adapter_summary)),
            0,
        );
        env.timeline_pts = Some(pts);
        Ok(env)
    }
}

// helpers ---------------------------------------------------------------------

fn reduced_den(a: Rational) -> i64 {
    let (n, d) = (a.num(), a.den());
    if n == 0 {
        return 1;
    }
    // Rational::new already normalizes, but reduce defensively — the bound
    // must hold on the REDUCED form regardless of construction path.
    let (mut n, mut d) = (n.abs(), d);
    while d != 0 {
        let t = n % d;
        n = d;
        d = t;
    }
    a.den() / n.max(1)
}

fn s_of(v: Option<SurfaceId>) -> SurfaceId {
    v.unwrap_or(SurfaceId::Output)
}
