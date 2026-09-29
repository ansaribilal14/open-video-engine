//! Wave 9 perf probe (BUILD_PLAN wave 6: "separate perf bench, committed
//! baselines, correctness ≠ performance").
//!
//! Renders the same multi-layer plan through both executors and reports
//! wall-clock per frame. HONESTY RULE (E-005 classification): when the
//! adapter is a software rasterizer (device_type Cpu / llvmpipe / lavapipe),
//! the GPU timings are recorded as **INVALID for any performance claim** —
//! the probe stamps every line with the adapter class. Real-hardware ratio
//! gates (testing-strategy T-7) remain a hardware-bound residual.

#[cfg(feature = "gpu")]
fn main() {
    use std::collections::HashMap;
    use std::time::Instant;

    use ove_media::{
        BackendId, BitDepth, ColorTags, FrameBytes, FrameEnvelope, PixelFormat, Primaries,
        StreamId, Transfer,
    };
    use ove_render::{
        compile_span, FrameSource, GpuRenderer, OutputSpec, Placement, RenderInput, RenderSpan,
        SoftwareRenderer, TrackInput,
    };
    use ove_time::Rational;

    let t = |n: i64, d: i64| Rational::new(n, d);
    let tags = ColorTags {
        primaries: Primaries::Bt709,
        transfer: Transfer::Bt709,
        matrix: ove_media::MatrixCoeffs::Bt709,
        range: ove_media::Range::Limited,
        chroma_loc: None,
    };

    fn rational(n: i64, d: i64) -> Rational {
        Rational::new(n, d)
    }

    struct Gradient(u32, u32, ColorTags);
    impl FrameSource for Gradient {
        fn fetch(&self, _target: Rational) -> Option<FrameEnvelope> {
            let (w, h, tags) = (self.0, self.1, self.2);
            let t = rational;
            let mut data = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    data.extend_from_slice(&[
                        (x * 7 % 256) as u8,
                        (y * 13 % 256) as u8,
                        (x * y % 256) as u8,
                        ((x * 255) / w.max(1)).min(255) as u8,
                    ]);
                }
            }
            Some(FrameEnvelope::video_cpu(
                Rational::zero(30),
                t(1, 30),
                StreamId(0),
                w,
                h,
                PixelFormat::Rgba,
                BitDepth::B8,
                tags,
                FrameBytes {
                    data,
                    strides: vec![(w * 4) as usize],
                },
                true,
                BackendId::Other("synthetic".into()),
                0,
            ))
        }
    }

    // 640×360, three layers (fetch+transform+blend ×3) — the W6-style graph.
    let input = RenderInput {
        tracks: vec![
            TrackInput {
                placements: vec![Placement {
                    clip_id: 1,
                    window: ove_timeline::mapping::ClipWindow::new(t(0, 1), t(30, 30), t(0, 1)),
                    source: 1,
                    alpha: t(1, 1),
                    offset: (0, 0),
                    src_color: tags,
                }],
            },
            TrackInput {
                placements: vec![Placement {
                    clip_id: 2,
                    window: ove_timeline::mapping::ClipWindow::new(t(0, 1), t(30, 30), t(0, 1)),
                    source: 2,
                    alpha: t(2, 3),
                    offset: (137, 89),
                    src_color: tags,
                }],
            },
            TrackInput {
                placements: vec![Placement {
                    clip_id: 3,
                    window: ove_timeline::mapping::ClipWindow::new(t(0, 1), t(30, 30), t(0, 1)),
                    source: 3,
                    alpha: t(1, 2),
                    offset: (-64, 210),
                    src_color: tags,
                }],
            },
        ],
        output: OutputSpec {
            width: 640,
            height: 360,
            rate_num: 30,
            rate_den: 1,
            working_space: tags,
        },
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(1, Box::new(Gradient(640, 360, tags)));
    sources.insert(2, Box::new(Gradient(320, 200, tags)));
    sources.insert(3, Box::new(Gradient(256, 144, tags)));

    const FRAMES: u64 = 60;
    let span = RenderSpan {
        first: 0,
        count: FRAMES,
    };
    let plans = compile_span(&input, span).expect("compile");
    let borrowed: HashMap<u64, &dyn FrameSource> =
        sources.iter().map(|(k, v)| (*k, &**v)).collect();

    // software reference timing
    let sw = SoftwareRenderer::new(borrowed.clone());
    let t0 = Instant::now();
    for p in &plans {
        let _ = sw.execute_frame(p).expect("sw");
    }
    let sw_ms = t0.elapsed().as_millis() as f64 / FRAMES as f64;

    // GPU timing
    let gpu = match GpuRenderer::new() {
        Ok(g) => g,
        Err(e) => {
            println!("adapter: UNAVAILABLE ({e})");
            println!("gpu frame: n/a | software frame: {sw_ms:.3} ms");
            return;
        }
    };
    let t0 = Instant::now();
    for p in &plans {
        let _ = gpu.execute_frame(p, &borrowed).expect("gpu");
    }
    let gpu_ms = t0.elapsed().as_millis() as f64 / FRAMES as f64;

    let summary = gpu.adapter_summary().to_lowercase();
    let software_adapter = summary.contains("llvmpipe")
        || summary.contains("lavapipe")
        || summary.contains("cpu")
        || summary.contains("swiftshader");
    println!("adapter: {}", gpu.adapter_summary());
    println!(
        "adapter_class: {}",
        if software_adapter {
            "SOFTWARE — GPU timings INVALID for any perf claim (E-005 rule)"
        } else {
            "REAL HARDWARE — timings citable"
        }
    );
    println!("software frame: {sw_ms:.3} ms");
    println!("gpu frame:      {gpu_ms:.3} ms");
    println!(
        "ratio gpu/software: {:.2}x {}",
        sw_ms / gpu_ms.max(1e-9),
        if software_adapter {
            "(INVALID — software adapter)"
        } else {
            ""
        }
    );
}

#[cfg(not(feature = "gpu"))]
fn main() {
    println!("build with --features gpu");
}
