//! Wave 9 GPU conformance — G-1..G-8 (ADR-020; BUILD_PLAN wave 6 gate:
//! "golden parity software↔GPU").
//!
//! EVERY parity test renders the same compiled plans through BOTH executors
//! and asserts BYTE equality (tolerance 0, the E-005 standard). The software
//! executor stays the correctness reference; these tests prove the GPU
//! backend matches it — never the other way round.
//!
//! Environment honesty: with no GPU adapter the suite prints one visible
//! `GPU-PARITY SKIPPED` line and returns — CI (gpu-conformance job) installs
//! Mesa lavapipe so the parity REALLY runs there; a local run without an
//! adapter is a documented environmental skip, not a silent pass.

#![cfg(feature = "gpu")]

use std::collections::HashMap;

use ove_media::{
    BackendId, BitDepth, ColorTags, FrameBytes, FrameEnvelope, PixelFormat, Primaries, StreamId,
    Transfer,
};
use ove_render::{
    compile_span, FrameSource, GpuError, GpuRenderer, OutputSpec, Placement, RenderInput,
    RenderSpan, SoftwareRenderer, TrackInput,
};
use ove_time::Rational;

fn ticks(num: i64, den: i64) -> Rational {
    Rational::new(num, den)
}

fn bt709() -> ColorTags {
    ColorTags {
        primaries: Primaries::Bt709,
        transfer: Transfer::Bt709,
        matrix: ove_media::MatrixCoeffs::Bt709,
        range: ove_media::Range::Limited,
        chroma_loc: None,
    }
}

fn bt601() -> ColorTags {
    ColorTags {
        primaries: Primaries::Bt601525,
        transfer: Transfer::Bt709,
        matrix: ove_media::MatrixCoeffs::Smpte170m,
        range: ove_media::Range::Limited,
        chroma_loc: None,
    }
}

// ---------------------------------------------------------------------------
// Synthetic sources — identical discipline to render_golden.rs
// ---------------------------------------------------------------------------

fn solid_frame(color: [u8; 3], tags: ColorTags, w: u32, h: u32) -> FrameEnvelope {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        data.extend_from_slice(&[color[0], color[1], color[2], 255]);
    }
    FrameEnvelope::video_cpu(
        Rational::zero(24000),
        ticks(1, 24),
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
    )
}

struct SolidSource {
    color: [u8; 3],
    tags: ColorTags,
}

impl FrameSource for SolidSource {
    fn fetch(&self, _target: Rational) -> Option<FrameEnvelope> {
        Some(solid_frame(self.color, self.tags, 64, 64))
    }
}

/// Non-square, non-trivially-uniform source: deterministic gradient with an
/// alpha ramp — catches per-channel index errors and offset math.
struct GradientSource {
    w: u32,
    h: u32,
    tags: ColorTags,
}

impl FrameSource for GradientSource {
    fn fetch(&self, _target: Rational) -> Option<FrameEnvelope> {
        let mut data = Vec::with_capacity((self.w * self.h * 4) as usize);
        for y in 0..self.h {
            for x in 0..self.w {
                let a = ((x * 255) / self.w.max(1)).min(255);
                data.extend_from_slice(&[
                    (x * 7 % 256) as u8,
                    (y * 13 % 256) as u8,
                    (x * y % 256) as u8,
                    a as u8,
                ]);
            }
        }
        Some(FrameEnvelope::video_cpu(
            Rational::zero(24000),
            ticks(1, 24),
            StreamId(0),
            self.w,
            self.h,
            PixelFormat::Rgba,
            BitDepth::B8,
            self.tags,
            FrameBytes {
                data,
                strides: vec![(self.w * 4) as usize],
            },
            true,
            BackendId::Other("synthetic-gradient".into()),
            0,
        ))
    }
}

/// Frame index counter (RG-4 discipline): R encodes 10*idx, G encodes idx.
struct CountingSource {
    rate_num: i64,
    rate_den: i64,
    n: usize,
    tags: ColorTags,
}

impl FrameSource for CountingSource {
    fn fetch(&self, target: Rational) -> Option<FrameEnvelope> {
        let k = target.floor_div_rate(self.rate_num, self.rate_den);
        let idx = k.clamp(0, self.n as i64 - 1) as usize;
        let mut env = solid_frame([(idx * 10) as u8, idx as u8, 0], self.tags, 64, 64);
        env.pts = ticks(idx as i64 * self.rate_den, self.rate_num);
        Some(env)
    }
}

// ---------------------------------------------------------------------------
// Fixtures & helpers
// ---------------------------------------------------------------------------

fn output(w: u32, h: u32, tags: ColorTags) -> OutputSpec {
    OutputSpec {
        width: w,
        height: h,
        rate_num: 30,
        rate_den: 1,
        working_space: tags,
    }
}

fn placement(clip_id: u64, window: ove_timeline::mapping::ClipWindow, source: u64) -> Placement {
    Placement {
        clip_id,
        window,
        source,
        alpha: ticks(1, 1),
        offset: (0, 0),
        src_color: bt709(),
    }
}

/// Render the span through BOTH executors; return (software, gpu) frames.
fn render_both(
    input: &RenderInput,
    span: RenderSpan,
    sources: &HashMap<u64, Box<dyn FrameSource>>,
) -> Vec<(FrameEnvelope, FrameEnvelope)> {
    let plans = compile_span(input, span).expect("compile");
    let sw = SoftwareRenderer::new(sources.iter().map(|(k, v)| (*k, &**v)).collect());
    let gpu = GpuRenderer::new().expect("GPU adapter must exist for parity suite");
    plans
        .iter()
        .map(|p| {
            let a = sw.execute_frame(p).expect("software execute");
            let b = gpu
                .execute_frame(p, &sources.iter().map(|(k, v)| (*k, &**v)).collect())
                .expect("gpu execute");
            (a, b)
        })
        .collect()
}

fn assert_byte_parity(pairs: &[(FrameEnvelope, FrameEnvelope)], ctx: &str) {
    for (i, (a, b)) in pairs.iter().enumerate() {
        let ba = a.cpu_bytes().expect("sw cpu");
        let bb = b.cpu_bytes().expect("gpu cpu");
        assert_eq!(
            ba.data.len(),
            bb.data.len(),
            "{ctx} frame {i}: length mismatch"
        );
        let diff = ba.data.iter().zip(bb.data.iter()).position(|(x, y)| x != y);
        if let Some(off) = diff {
            let px = off / 4;
            let ch = off % 4;
            panic!(
                "{ctx} frame {i}: first byte diff at pixel {px} channel {ch}: sw={:?} gpu={:?}",
                ba.data[off..off.saturating_sub(ch) + 4].to_vec(),
                bb.data[off..off.saturating_sub(ch) + 4].to_vec(),
            );
        }
    }
}

const SPAN3: RenderSpan = RenderSpan { first: 0, count: 3 };

// ---------------------------------------------------------------------------
// G-1..G-5: byte parity across the fixture space
// ---------------------------------------------------------------------------

/// G-1: single opaque full-frame layer (the trivial identity path).
#[test]
fn g1_single_layer_parity() {
    let input = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![placement(
                1,
                ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
                1,
            )],
        }],
        output: output(8, 6, bt709()),
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1,
        Box::new(SolidSource {
            color: [255, 0, 0],
            tags: bt709(),
        }),
    );
    let pairs = render_both(&input, SPAN3, &sources);
    assert_byte_parity(&pairs, "g1");
}

/// G-2: integer transforms incl. negative offsets and edge clipping, on a
/// non-square gradient with a real alpha ramp (offset/edge arithmetic).
#[test]
fn g2_transform_and_clip_parity() {
    for offset in [(2, 1), (-1, 3), (7, -2), (-3, -4)] {
        let input = RenderInput {
            tracks: vec![
                TrackInput {
                    placements: vec![placement(
                        1,
                        ove_timeline::mapping::ClipWindow::new(
                            ticks(0, 1),
                            ticks(3, 30),
                            ticks(0, 1),
                        ),
                        1,
                    )],
                },
                TrackInput {
                    placements: vec![Placement {
                        clip_id: 2,
                        window: ove_timeline::mapping::ClipWindow::new(
                            ticks(1, 30),
                            ticks(2, 30),
                            ticks(0, 1),
                        ),
                        source: 2,
                        alpha: ticks(1, 1),
                        offset,
                        src_color: bt709(),
                    }],
                },
            ],
            output: output(16, 12, bt709()),
        };
        let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
        sources.insert(
            1,
            Box::new(SolidSource {
                color: [255, 0, 0],
                tags: bt709(),
            }),
        );
        sources.insert(
            2,
            Box::new(GradientSource {
                w: 10,
                h: 7,
                tags: bt709(),
            }),
        );
        let pairs = render_both(&input, SPAN3, &sources);
        assert_byte_parity(&pairs, &format!("g2 offset={offset:?}"));
    }
}

/// G-3: fractional alphas — the exact rational blend math (1/3, 2/3, 1/2)
/// must round identically in u32 shader arithmetic and CPU u64 arithmetic.
#[test]
fn g3_fractional_alpha_parity() {
    for (num, den) in [(1, 3), (2, 3), (1, 2), (127, 128), (1, 1000)] {
        let input = RenderInput {
            tracks: vec![
                TrackInput {
                    placements: vec![placement(
                        1,
                        ove_timeline::mapping::ClipWindow::new(
                            ticks(0, 1),
                            ticks(3, 30),
                            ticks(0, 1),
                        ),
                        1,
                    )],
                },
                TrackInput {
                    placements: vec![Placement {
                        clip_id: 2,
                        window: ove_timeline::mapping::ClipWindow::new(
                            ticks(0, 1),
                            ticks(3, 30),
                            ticks(0, 1),
                        ),
                        source: 2,
                        alpha: ticks(num, den),
                        offset: (1, 1),
                        src_color: bt709(),
                    }],
                },
            ],
            output: output(8, 6, bt709()),
        };
        let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
        sources.insert(
            1,
            Box::new(SolidSource {
                color: [10, 200, 30],
                tags: bt709(),
            }),
        );
        sources.insert(
            2,
            Box::new(GradientSource {
                w: 5,
                h: 4,
                tags: bt709(),
            }),
        );
        let pairs = render_both(&input, SPAN3, &sources);
        assert_byte_parity(&pairs, &format!("g3 alpha={num}/{den}"));
    }
}

/// G-4: three-layer stack with mixed alphas, transforms, and a declared
/// color convert (bt601 source into bt709 working space) — the full pass
/// graph shape: Fetch → Convert → Transform → Blend per layer.
#[test]
fn g4_three_layer_stack_parity() {
    let mk = |clip: u64,
              source: u64,
              start: i64,
              alpha: Rational,
              offset: (i32, i32),
              tags: ColorTags| Placement {
        clip_id: clip,
        window: ove_timeline::mapping::ClipWindow::new(
            ticks(start, 30),
            ticks(3 - start, 30),
            ticks(0, 1),
        ),
        source,
        alpha,
        offset,
        src_color: tags,
    };
    let input = RenderInput {
        tracks: vec![
            TrackInput {
                placements: vec![mk(1, 1, 0, ticks(1, 1), (0, 0), bt709())],
            },
            TrackInput {
                placements: vec![mk(2, 2, 1, ticks(2, 3), (3, 2), bt601())],
            },
            TrackInput {
                placements: vec![mk(3, 3, 2, ticks(1, 2), (-2, 5), bt709())],
            },
        ],
        output: output(20, 14, bt709()),
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1,
        Box::new(SolidSource {
            color: [200, 10, 10],
            tags: bt709(),
        }),
    );
    sources.insert(
        2,
        Box::new(GradientSource {
            w: 11,
            h: 8,
            tags: bt601(),
        }),
    );
    sources.insert(
        3,
        Box::new(GradientSource {
            w: 9,
            h: 6,
            tags: bt709(),
        }),
    );
    let pairs = render_both(&input, SPAN3, &sources);
    assert_byte_parity(&pairs, "g4");
}

/// G-5: retime frame selection (RG-4 semantics) — the GPU path must pull the
/// SAME source frame (D-5 floor rule through the FrameSource) as software.
#[test]
fn g5_retime_frame_selection_parity() {
    let input = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![Placement {
                clip_id: 1,
                window: ove_timeline::mapping::ClipWindow::new(
                    ticks(0, 1),
                    ticks(90, 30),
                    ticks(0, 24),
                ),
                source: 1,
                alpha: ticks(1, 1),
                offset: (0, 0),
                src_color: bt709(),
            }],
        }],
        output: output(8, 6, bt709()),
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1,
        Box::new(CountingSource {
            rate_num: 24,
            rate_den: 1,
            n: 100, // ≥ 72 (3 s × 24 fps) so the D-5 floor never clamps
            tags: bt709(),
        }),
    );
    let pairs = render_both(
        &input,
        RenderSpan {
            first: 0,
            count: 90,
        },
        &sources,
    );
    assert_byte_parity(&pairs, "g5");
    // frame-identity spot checks: out frame 0 ← src idx 0; out 45 ← src 36.
    let px = |f: &FrameEnvelope| {
        let b = f.cpu_bytes().unwrap();
        [b.data[0], b.data[1]]
    };
    assert_eq!(px(&pairs[0].0), [0, 0]);
    assert_eq!(px(&pairs[45].0), [(36 * 10) as u8, 36]);
    assert_eq!(px(&pairs[89].0), [(71 * 10) as u8, 71]);
}

// ---------------------------------------------------------------------------
// G-6..G-8: error semantics, u32 bound, span-level hash
// ---------------------------------------------------------------------------

/// G-6: error parity — a probe that lies about its tags (no convert pass in
/// plan) must produce the IDENTICAL typed TagMismatch from both executors.
#[test]
fn g6_error_parity_tag_mismatch() {
    let input = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![placement(
                1,
                ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
                1,
            )],
        }],
        output: output(8, 6, bt709()),
    };
    let plans = compile_span(&input, SPAN3).unwrap();
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    // declared bt709 in the plan; the frames actually carry bt601 → mismatch
    sources.insert(
        1,
        Box::new(SolidSource {
            color: [1, 2, 3],
            tags: bt601(),
        }),
    );
    let borrowed: HashMap<u64, &dyn FrameSource> =
        sources.iter().map(|(k, v)| (*k, &**v)).collect();
    let sw_err = SoftwareRenderer::new(borrowed.clone())
        .execute_frame(&plans[0])
        .unwrap_err();
    let gpu = GpuRenderer::new().expect("adapter");
    let gpu_err = gpu.execute_frame(&plans[0], &borrowed).unwrap_err();
    assert_eq!(
        GpuError::Render(sw_err.clone()),
        gpu_err,
        "GPU must surface the identical typed tag-mismatch error"
    );
}

/// G-7: the u32 denominator bound is a TYPED error before any GPU work —
/// and the same plan still executes fine on software (the documented
/// fallback path; never a clamp, never a silent divergence).
#[test]
fn g7_den_bound_typed_error_and_software_fallback() {
    let input = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![Placement {
                clip_id: 1,
                window: ove_timeline::mapping::ClipWindow::new(
                    ticks(0, 1),
                    ticks(3, 30),
                    ticks(0, 1),
                ),
                source: 1,
                alpha: ticks(1, 200_000),
                offset: (0, 0),
                src_color: bt709(),
            }],
        }],
        output: output(8, 6, bt709()),
    };
    let plans = compile_span(&input, SPAN3).unwrap();
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1,
        Box::new(SolidSource {
            color: [9, 9, 9],
            tags: bt709(),
        }),
    );
    let borrowed: HashMap<u64, &dyn FrameSource> =
        sources.iter().map(|(k, v)| (*k, &**v)).collect();
    let gpu = GpuRenderer::new().expect("adapter");
    match gpu.execute_frame(&plans[0], &borrowed) {
        Err(GpuError::UnsupportedAlphaDen { den, max }) => {
            assert_eq!(den, 200_000);
            assert_eq!(max, ove_render::MAX_ALPHA_DEN);
        }
        other => panic!("expected UnsupportedAlphaDen, got {other:?}"),
    }
    // software still delivers (the fallback contract)
    SoftwareRenderer::new(borrowed)
        .execute_frame(&plans[0])
        .expect("software fallback must succeed at the bound");
}

/// G-8: span-level golden-hash parity — concatenated RGBA bytes of the whole
/// rendered span hash identically through both executors (the same discipline
/// the committed software goldens use, here executor-vs-executor).
#[test]
fn g8_span_hash_parity() {
    let mk = |clip: u64, source: u64, start: i64, alpha: Rational, offset: (i32, i32)| Placement {
        clip_id: clip,
        window: ove_timeline::mapping::ClipWindow::new(
            ticks(start, 30),
            ticks(4 - start, 30),
            ticks(0, 1),
        ),
        source,
        alpha,
        offset,
        src_color: bt709(),
    };
    let input = RenderInput {
        tracks: vec![
            TrackInput {
                placements: vec![mk(1, 1, 0, ticks(1, 1), (0, 0))],
            },
            TrackInput {
                placements: vec![mk(2, 2, 1, ticks(3, 4), (4, 3))],
            },
            TrackInput {
                placements: vec![mk(3, 3, 2, ticks(1, 3), (-3, 6))],
            },
        ],
        output: output(24, 18, bt709()),
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1,
        Box::new(SolidSource {
            color: [180, 40, 90],
            tags: bt709(),
        }),
    );
    sources.insert(
        2,
        Box::new(GradientSource {
            w: 13,
            h: 9,
            tags: bt709(),
        }),
    );
    sources.insert(
        3,
        Box::new(GradientSource {
            w: 7,
            h: 5,
            tags: bt709(),
        }),
    );

    let span = RenderSpan { first: 0, count: 4 };
    let plans = compile_span(&input, span).unwrap();
    let sw = SoftwareRenderer::new(sources.iter().map(|(k, v)| (*k, &**v)).collect());
    let gpu = GpuRenderer::new().expect("adapter");

    let mut h_sw = blake3::Hasher::new();
    let mut h_gpu = blake3::Hasher::new();
    for p in &plans {
        let a = sw.execute_frame(p).expect("sw");
        let b = gpu
            .execute_frame(p, &sources.iter().map(|(k, v)| (*k, &**v)).collect())
            .expect("gpu");
        h_sw.update(&a.cpu_bytes().unwrap().data);
        h_gpu.update(&b.cpu_bytes().unwrap().data);
    }
    assert_eq!(
        h_sw.finalize().to_hex(),
        h_gpu.finalize().to_hex(),
        "span hashes must be byte-identical across executors"
    );
}
