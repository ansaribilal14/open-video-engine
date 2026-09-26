//! E-005 correctness leg: wgpu headless YUV420 -> RGB (BT.601 limited, integer-exact)
//! composite of two "decoded frames" (nearest-neighbor scale + over), full readback,
//! pixel-exact comparison against an identical host reference.
//!
//! Runs on software Vulkan (Mesa lavapipe) extracted rootless via `apt download`.
//! Classification rules for the record:
//!   - correctness leg  : RUN (pixel-exact or FAIL)
//!   - performance leg  : INVALID (software renderer; numbers recorded, never cited)
//!   - zero-copy import : NOT RUN (real-GPU/driver bound; browser leg = E-001)

use std::sync::mpsc;

const OUT_W: u32 = 64;
const OUT_H: u32 = 64;
const B_SRC_W: u32 = 16; // frame B source size
const B_SRC_H: u32 = 16;
const B_DST_X: u32 = 24; // frame B dest rect in output
const B_DST_Y: u32 = 24;
const B_DST_W: u32 = 32;
const B_DST_H: u32 = 32;

struct Frame {
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    w: u32,
    h: u32,
}

/// Procedural, deterministic "decoded frames" (stand-ins for real decoder output).
/// Frame A: horizontal+vertical gradient with distinct chroma blocks.
/// Frame B: checkerboard + radial luma ramp (distinct from A everywhere it matters).
fn make_frame_a(w: u32, h: u32) -> Frame {
    let mut y = Vec::with_capacity((w * h) as usize);
    let mut u = Vec::new();
    let mut v = Vec::new();
    for py in 0..h {
        for px in 0..w {
            y.push(((px * 3 + py * 5) % 256) as u8);
        }
    }
    for py in 0..h / 2 {
        for px in 0..w / 2 {
            u.push(if (px / 4 + py / 4) % 2 == 0 { 90 } else { 160 });
            v.push(if (px / 6 + py / 2) % 2 == 0 { 200 } else { 60 });
        }
    }
    Frame { y, u, v, w, h }
}

fn make_frame_b(w: u32, h: u32) -> Frame {
    let mut y = Vec::with_capacity((w * h) as usize);
    let mut u = Vec::new();
    let mut v = Vec::new();
    for py in 0..h {
        for px in 0..w {
            let d = (px as i32 - w as i32 / 2).abs() + (py as i32 - h as i32 / 2).abs();
            y.push((16 + d * 12).clamp(16, 235) as u8);
        }
    }
    for py in 0..h / 2 {
        for px in 0..w / 2 {
            u.push(if (px + py) % 2 == 0 { 128 } else { 40 });
            v.push(if px % 3 == 0 { 240 } else { 110 });
        }
    }
    Frame { y, u, v, w, h }
}

/// Host reference: EXACT same integer math as the WGSL shader (BT.601 limited range).
fn yuv_to_rgb8(y: u8, u: u8, v: u8) -> [u8; 3] {
    let yy = y as i32 - 16;
    let uu = u as i32 - 128;
    let vv = v as i32 - 128;
    let r = ((298 * yy + 409 * vv + 128) >> 8).clamp(0, 255);
    let g = ((298 * yy - 100 * uu - 208 * vv + 128) >> 8).clamp(0, 255);
    let b = ((298 * yy + 516 * uu + 128) >> 8).clamp(0, 255);
    [r as u8, g as u8, b as u8]
}

/// Host-side expected output: composite B (nearest-neighbor x2) over A.
fn expected_output(a: &Frame, b: &Frame) -> Vec<u8> {
    let mut out = vec![0u8; (OUT_W * OUT_H * 4) as usize];
    for py in 0..OUT_H {
        for px in 0..OUT_W {
            let in_rect = px >= B_DST_X && px < B_DST_X + B_DST_W && py >= B_DST_Y && py < B_DST_Y + B_DST_H;
            let idx = ((py * OUT_W + px) * 4) as usize;
            let rgb8 = if in_rect {
                let lx = px - B_DST_X;
                let ly = py - B_DST_Y;
                // nearest-neighbor: src = local * src_size / dst_size (integer floor)
                let sx = (lx * B_SRC_W / B_DST_W).min(b.w - 1);
                let sy = (ly * B_SRC_H / B_DST_H).min(b.h - 1);
                yuv_to_rgb8(b.y[(sy * b.w + sx) as usize], b.u[((sy / 2) * (b.w / 2) + sx / 2) as usize], b.v[((sy / 2) * (b.w / 2) + sx / 2) as usize])
            } else {
                yuv_to_rgb8(a.y[(py * a.w + px) as usize], a.u[((py / 2) * (a.w / 2) + px / 2) as usize], a.v[((py / 2) * (a.w / 2) + px / 2) as usize])
            };
            out[idx] = rgb8[0];
            out[idx + 1] = rgb8[1];
            out[idx + 2] = rgb8[2];
            out[idx + 3] = 255;
        }
    }
    out
}

const SHADER: &str = r#"
struct Params {
    rect_origin: vec2u,
    rect_size: vec2u,
    out_size: vec2u,
    pad: vec2u,
};

@group(0) @binding(0) var texA_y: texture_2d<u32>;
@group(0) @binding(1) var texA_u: texture_2d<u32>;
@group(0) @binding(2) var texA_v: texture_2d<u32>;
@group(0) @binding(3) var texB_y: texture_2d<u32>;
@group(0) @binding(4) var texB_u: texture_2d<u32>;
@group(0) @binding(5) var texB_v: texture_2d<u32>;
@group(0) @binding(6) var<uniform> p: Params;

fn yuv_to_rgb8(y: u32, u: u32, v: u32) -> vec3<u32> {
    let yy = i32(y) - 16;
    let uu = i32(u) - 128;
    let vv = i32(v) - 128;
    let r = clamp((298 * yy + 409 * vv + 128) >> 8, 0, 255);
    let g = clamp((298 * yy - 100 * uu - 208 * vv + 128) >> 8, 0, 255);
    let b = clamp((298 * yy + 516 * uu + 128) >> 8, 0, 255);
    return vec3<u32>(u32(r), u32(g), u32(b));
}

fn sample_yuv(ty: texture_2d<u32>, tu: texture_2d<u32>, tv: texture_2d<u32>, c: vec2u) -> vec3<u32> {
    let dims = textureDimensions(ty);
    let cc = min(c, vec2u(dims) - vec2u(1u));
    let y = textureLoad(ty, cc, 0).r;
    let u = textureLoad(tu, cc / 2u, 0).r;
    let v = textureLoad(tv, cc / 2u, 0).r;
    return yuv_to_rgb8(y, u, v);
}

struct VSOut {
    @builtin(position) pos: vec4f,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VSOut {
    var pos = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    var out: VSOut;
    out.pos = vec4f(pos[vi], 0.0, 1.0);
    return out;
}

@fragment
fn fs(@builtin(position) fpos: vec4f) -> @location(0) vec4f {
    let coord = min(vec2u(floor(fpos.xy)), p.out_size - vec2u(1u));
    let in_rect = all(coord >= p.rect_origin) && all(coord < p.rect_origin + p.rect_size);
    var rgb8: vec3<u32>;
    if p.pad.x == 1u {
        // DIAGNOSTIC passthrough: expose raw Y,U,V the shader sees at each pixel
        let dims = textureDimensions(texA_y);
        let cc = min(coord, vec2u(dims) - vec2u(1u));
        rgb8 = vec3<u32>(textureLoad(texA_y, cc, 0).r, textureLoad(texA_u, cc / 2u, 0).r, textureLoad(texA_v, cc / 2u, 0).r);
    } else if in_rect {
        let local = coord - p.rect_origin;
        let src = min((local * vec2u(${B_SRC_W}u, ${B_SRC_H}u)) / p.rect_size, vec2u(${B_SRC_W}u, ${B_SRC_H}u) - vec2u(1u));
        rgb8 = sample_yuv(texB_y, texB_u, texB_v, src);
    } else {
        rgb8 = sample_yuv(texA_y, texA_u, texA_v, coord);
    }
    return vec4f(f32(rgb8.r) / 255.0, f32(rgb8.g) / 255.0, f32(rgb8.b) / 255.0, 1.0);
}
"#;

fn main() {
    let mut report = String::new();
    let t0 = std::time::Instant::now();

    // --- adapter ---
    let mut desc = wgpu::InstanceDescriptor::default();
    desc.backends = wgpu::Backends::VULKAN; // force Vulkan -> our rootless lavapipe ICD
    let instance = wgpu::Instance::new(&desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("E-005: no Vulkan adapter (lavapipe ICD not visible — check VK_ICD_FILENAMES/LD_LIBRARY_PATH)");
    let info = adapter.get_info();
    let is_lavapipe = info.name.to_lowercase().contains("llvmpipe") || info.name.to_lowercase().contains("lavapipe");
    report.push_str(&format!("adapter: name=\"{}\" driver=\"{}\" backend={:?} device_type={:?}\n", info.name, info.driver, info.backend, info.device_type));
    report.push_str(&format!("adapter_class: {}\n", if is_lavapipe { "SOFTWARE (lavapipe) — perf leg INVALID by definition" } else { "NON-SOFTWARE — unexpected in this sandbox; investigate" }));

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("e005"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: Default::default(),
    }))
    .expect("E-005: request_device failed");
    report.push_str(&format!("device acquired in {:?}\n", t0.elapsed()));

    // --- frames + upload (copy path: queue.write_texture — the V1 universal path per ADR-004) ---
    let a = make_frame_a(OUT_W, OUT_H);
    let b = make_frame_b(B_SRC_W, B_SRC_H);
    let planar = |f: &Frame| (f.y.clone(), f.u.clone(), f.v.clone());
    let (ay, au, av) = planar(&a);
    let (by, bu, bv) = planar(&b);

    let mk_tex = |w: u32, h: u32, data: &[u8], label: &'static str| {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        tex.create_view(&wgpu::TextureViewDescriptor::default())
    };

    let vay = mk_tex(OUT_W, OUT_H, &ay, "A_y");
    let vau = mk_tex(OUT_W / 2, OUT_H / 2, &au, "A_u");
    let vav = mk_tex(OUT_W / 2, OUT_H / 2, &av, "A_v");
    let vby = mk_tex(B_SRC_W, B_SRC_H, &by, "B_y");
    let vbu = mk_tex(B_SRC_W / 2, B_SRC_H / 2, &bu, "B_u");
    let vbv = mk_tex(B_SRC_W / 2, B_SRC_H / 2, &bv, "B_v");

    // uniform: [origin(2) size(2) out(2) pad(2)] as u32 pairs; pad.x=1 => diagnostic passthrough
    let diag: u32 = std::env::var("E005_DIAG").map(|v| v == "1").unwrap_or(false) as u32;
    let uniform_bytes: [u8; 32] = [
        B_DST_X, B_DST_Y, B_DST_W, B_DST_H, OUT_W, OUT_H, diag, 0,
    ].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>().try_into().unwrap();
    let ubuf = wgpu::util::DeviceExt::create_buffer_init(
        &device,
        &wgpu::util::BufferInitDescriptor { label: Some("params"), contents: &uniform_bytes, usage: wgpu::BufferUsages::UNIFORM },
    );

    let shader_src = SHADER.replace("${B_SRC_W}", &B_SRC_W.to_string()).replace("${B_SRC_H}", &B_SRC_H.to_string());
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("e005_fs"), source: wgpu::ShaderSource::Wgsl(shader_src.into()) });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    let playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("pl"), bind_group_layouts: &[&bgl], push_constant_ranges: &[] });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("e005_pipeline"),
        layout: Some(&playout),
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&vay) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&vau) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&vav) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&vby) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&vbu) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&vbv) },
            wgpu::BindGroupEntry { binding: 6, resource: ubuf.as_entire_binding() },
        ],
    });

    let out_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("out"),
        size: wgpu::Extent3d { width: OUT_W, height: OUT_H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let out_view = out_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let bytes_per_row = OUT_W * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bytes_per_row * OUT_H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // --- encode + submit ---
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("e005_enc") });
    {
        let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("e005_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &out_view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&pipeline);
        rpass.set_bind_group(0, &bind, &[]);
        rpass.draw(0..3, 0..1);
    }
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &out_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: Some(OUT_H) },
        },
        wgpu::Extent3d { width: OUT_W, height: OUT_H, depth_or_array_layers: 1 },
    );
    queue.submit([enc.finish()]);

    // --- readback ---
    let (tx, rx) = mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |res| { let _ = tx.send(res); });
    let _ = device.poll(wgpu::PollType::Wait);
    rx.recv().expect("map callback not invoked").expect("map failed");
    {
        let view = readback.get_mapped_range(..);
        let got: Vec<u8> = view.chunks(bytes_per_row as usize).flat_map(|c| &c[..(OUT_W * 4) as usize]).cloned().collect();
        let expected = expected_output(&a, &b);
        let mismatches: Vec<(u32, u32, [u8; 4], [u8; 4])> = (0..OUT_H)
            .flat_map(|py| (0..OUT_W).map(move |px| (py, px)))
            .filter_map(|(py, px)| {
                let i = ((py * OUT_W + px) * 4) as usize;
                let g = [got[i], got[i + 1], got[i + 2], got[i + 3]];
                let e = [expected[i], expected[i + 1], expected[i + 2], expected[i + 3]];
                if g != e { Some((px, py, g, e)) } else { None }
            })
            .collect();
        let total = OUT_W * OUT_H;
        let ok = mismatches.is_empty();
        report.push_str(&format!(
            "composite: {} px checked, {} exact-match, {} mismatch (tolerance=0)\n",
            total,
            total - mismatches.len() as u32,
            mismatches.len()
        ));
        if !ok {
            for (px, py, g, e) in mismatches.iter().take(8) {
                report.push_str(&format!("  MISMATCH @({px},{py}): got {g:?} expected {e:?}\n"));
            }
        }
        report.push_str(&format!("verdict: {}\n", if ok { "PASS" } else { "FAIL" }));
    }
    drop(readback);

    // second run timing (still software — recorded, NEVER cited as GPU perf)
    let t1 = std::time::Instant::now();
    for _ in 0..10 {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: None, color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &out_view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &bind, &[]);
            rpass.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
    }
    let _ = device.poll(wgpu::PollType::Wait);
    report.push_str(&format!("timing_software_only: 10 renders in {:?} (INVALID for any perf claim — llvmpipe, not GPU)\n", t1.elapsed()));
    report.push_str(&format!("total_wall: {:?}\n", t0.elapsed()));

    println!("{report}");
    std::fs::write("/home/z/my-project/open-video-engine/experiments/E-005_result.txt", &report).unwrap();
    if !report.contains("verdict: PASS") {
        std::process::exit(1);
    }
}
