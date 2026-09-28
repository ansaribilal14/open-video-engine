//! ove-render correctness suite — RG-1..RG-7 (RENDER_GRAPH_SPEC §4).
//!
//! Golden policy: hashes are over CONCATENATED RGBA bytes of every rendered
//! frame (opaque output, working-space tags) — tiny frames (8×6) so a human
//! can hand-verify pixels; the semantic tests additionally assert the exact
//! math (blend values, retime frame indices) so a wrong-but-stable renderer
//! cannot pass. A golden hash failure prints the actual hash for an
//! INTENTIONAL diff — never weaken an assertion to go green.

use std::collections::HashMap;

use ove_media::{
    BackendId, BitDepth, ColorTags, FrameBytes, FrameEnvelope, PixelFormat, Primaries, StreamId,
    Transfer,
};
use ove_render::{
    compile_frame, compile_span, FrameSource, OutputSpec, Placement, RenderError, RenderInput,
    RenderSpan, SoftwareRenderer, TrackInput,
};
use ove_time::Rational;
use ove_timeline::{Clip, Command, GapTrack, Timeline, TrackKind};

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
// Synthetic sources (Cpu RGBA8, opaque, declared tags) — deterministic
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

/// Solid color at any requested time.
struct SolidSource {
    color: [u8; 3],
    tags: ColorTags,
}

impl FrameSource for SolidSource {
    fn fetch(&self, _target: Rational) -> Option<FrameEnvelope> {
        Some(solid_frame(self.color, self.tags, 64, 64))
    }
}

/// Frame index counter: R encodes 10*idx, G encodes idx — retime verification
/// reads WHICH source frame landed in the output (RG-4).
struct CountingSource {
    rate_num: i64,
    rate_den: i64,
    n: usize,
    tags: ColorTags,
}

impl CountingSource {
    fn frame_pts(&self, idx: usize) -> Rational {
        ticks(idx as i64 * self.rate_den, self.rate_num)
    }
}

impl FrameSource for CountingSource {
    fn fetch(&self, target: Rational) -> Option<FrameEnvelope> {
        // greatest idx with idx/rate <= target  (D-5 floor rule)
        let k = target.floor_div_rate(self.rate_num, self.rate_den);
        let idx = k.clamp(0, self.n as i64 - 1) as usize;
        let mut env = solid_frame([(idx * 10) as u8, idx as u8, 0], self.tags, 64, 64);
        env.pts = self.frame_pts(idx);
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

fn render_all(
    input: &RenderInput,
    span: RenderSpan,
    sources: &HashMap<u64, Box<dyn FrameSource>>,
) -> Vec<FrameEnvelope> {
    let plans = compile_span(input, span).expect("compile");
    let renderer = SoftwareRenderer::new(sources.iter().map(|(k, v)| (*k, &**v)).collect());
    plans
        .iter()
        .map(|p| renderer.execute_frame(p).expect("execute"))
        .collect()
}

fn golden_hash(frames: &[FrameEnvelope]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(frames.len() as u64).to_le_bytes());
    for f in frames {
        let bytes = f.cpu_bytes().expect("cpu frame");
        hasher.update(&(f.width).to_le_bytes());
        hasher.update(&(f.height).to_le_bytes());
        hasher.update(&bytes.data);
    }
    hasher.finalize().to_hex()[..32].to_string()
}

fn px(frame: &FrameEnvelope, x: u32, y: u32) -> [u8; 4] {
    let bytes = frame.cpu_bytes().expect("cpu");
    let stride = bytes.strides[0];
    let off = (y as usize) * stride + (x as usize) * 4;
    [
        bytes.data[off],
        bytes.data[off + 1],
        bytes.data[off + 2],
        bytes.data[off + 3],
    ]
}

const SPAN3: RenderSpan = RenderSpan { first: 0, count: 3 };

// ---------------------------------------------------------------------------
// RG-1: compile purity — canonical plan hashes equal across runs and threads
// ---------------------------------------------------------------------------

#[test]
fn rg1_compile_purity() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }
    let build = |seed: u64| -> RenderInput {
        let mut rng = Rng(seed);
        let mut tracks = Vec::new();
        for _ in 0..3 {
            let mut placements = Vec::new();
            for cid in 1..6u64 {
                let start = ticks((rng.next() % 40) as i64, 30);
                let dur = ticks(1 + (rng.next() % 30) as i64, 30);
                placements.push(Placement {
                    clip_id: cid,
                    window: ove_timeline::mapping::ClipWindow::new(
                        start,
                        dur,
                        ticks((rng.next() % 20) as i64, 24),
                    ),
                    source: 1,
                    alpha: ticks((rng.next() % 3) as i64, 2),
                    offset: ((rng.next() % 3) as i32 - 1, (rng.next() % 3) as i32 - 1),
                    src_color: bt709(),
                });
            }
            placements.sort_by_key(|p| (p.window.timeline_start, p.clip_id));
            tracks.push(TrackInput { placements });
        }
        RenderInput {
            tracks,
            output: output(8, 6, bt709()),
        }
    };
    for seed in [0xDE, 0xAD, 0xBE, 0xEF] {
        let input = build(seed);
        let span = RenderSpan {
            first: 0,
            count: 10,
        };
        let plans_a = compile_span(&input, span).unwrap();
        let plans_b = compile_span(&input, span).unwrap();
        let ha: Vec<u64> = plans_a.iter().map(|p| p.canonical_hash()).collect();
        let hb: Vec<u64> = plans_b.iter().map(|p| p.canonical_hash()).collect();
        assert_eq!(ha, hb, "same input must compile identically (seed {seed})");
        // cross-thread purity
        let t = std::thread::spawn(move || {
            compile_span(&input, span)
                .unwrap()
                .iter()
                .map(|p| p.canonical_hash())
                .collect::<Vec<u64>>()
        });
        assert_eq!(t.join().unwrap(), ha, "thread purity (seed {seed})");
    }
}

// ---------------------------------------------------------------------------
// RG-2: golden frames — fixture set, committed hashes
// ---------------------------------------------------------------------------

/// Fixture A: single opaque red layer, 3 frames.
fn fixture_a() -> (RenderInput, HashMap<u64, Box<dyn FrameSource>>) {
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
        1u64,
        Box::new(SolidSource {
            color: [255, 0, 0],
            tags: bt709(),
        }),
    );
    (input, sources)
}

/// Fixture B: two layers overlapping — bottom red full-frame, top green
/// translated (2,1); frames 0-2 with the top layer active from frame 1.
fn fixture_b() -> (RenderInput, HashMap<u64, Box<dyn FrameSource>>) {
    let input = RenderInput {
        tracks: vec![
            TrackInput {
                placements: vec![placement(
                    1,
                    ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
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
                    offset: (2, 1),
                    src_color: bt709(),
                }],
            },
        ],
        output: output(8, 6, bt709()),
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1u64,
        Box::new(SolidSource {
            color: [255, 0, 0],
            tags: bt709(),
        }),
    );
    sources.insert(
        2u64,
        Box::new(SolidSource {
            color: [0, 255, 0],
            tags: bt709(),
        }),
    );
    (input, sources)
}

/// Fixture C: 50% blue over red (the hand-verifiable blend).
fn fixture_c() -> (RenderInput, HashMap<u64, Box<dyn FrameSource>>) {
    let mut p = placement(
        2,
        ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
        2,
    );
    p.alpha = ticks(1, 2);
    let input = RenderInput {
        tracks: vec![
            TrackInput {
                placements: vec![placement(
                    1,
                    ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
                    1,
                )],
            },
            TrackInput {
                placements: vec![p],
            },
        ],
        output: output(8, 6, bt709()),
    };
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1u64,
        Box::new(SolidSource {
            color: [255, 0, 0],
            tags: bt709(),
        }),
    );
    sources.insert(
        2u64,
        Box::new(SolidSource {
            color: [0, 0, 255],
            tags: bt709(),
        }),
    );
    (input, sources)
}

#[test]
fn rg2_golden_frames() {
    // Fixture A: pure red everywhere.
    let (input, sources) = fixture_a();
    let frames = render_all(&input, SPAN3, &sources);
    for f in &frames {
        assert_eq!(px(f, 0, 0), [255, 0, 0, 255]);
        assert_eq!(px(f, 7, 5), [255, 0, 0, 255]);
    }
    let golden_a = golden_hash(&frames);

    // Fixture B: frame 0 = pure red; frames 1-2 = green translated (2,1) over
    // red — pixel (2,1) is green, pixel (0,0) stays red (top layer's origin
    // sits at (2,1) so (0,0) is untouched), pixel (1,1) stays red.
    let (input, sources) = fixture_b();
    let frames = render_all(&input, SPAN3, &sources);
    assert_eq!(
        px(&frames[0], 2, 1),
        [255, 0, 0, 255],
        "top layer inactive at frame 0"
    );
    assert_eq!(px(&frames[1], 2, 1), [0, 255, 0, 255], "top layer origin");
    assert_eq!(
        px(&frames[1], 0, 0),
        [255, 0, 0, 255],
        "outside translated rect"
    );
    assert_eq!(px(&frames[1], 1, 1), [255, 0, 0, 255], "rect starts at x=2");
    let golden_b = golden_hash(&frames);

    // Fixture C: 50% blue over red — exact integer blend:
    // out = (c_s*255 + c_d*255 + 255)/510 → 127.5 rounds UP to 128.
    let (input, sources) = fixture_c();
    let frames = render_all(&input, SPAN3, &sources);
    assert_eq!(
        px(&frames[0], 0, 0),
        [128, 0, 128, 255],
        "50% blue over red = (128,0,128)"
    );
    let golden_c = golden_hash(&frames);

    // The committed goldens (RENDER_GRAPH_SPEC §4 RG-2). Changing any of
    // these REQUIRES a byte-level explanation of what changed and why.
    // Committed goldens (RENDER_GRAPH_SPEC §4 RG-2). Hand-verified pixel
    // semantics live in the asserts above; these hashes pin ALL bytes. Any
    // change REQUIRES a byte-level explanation (intentional golden diff).
    assert_eq!(golden_a, "34937a65217ad69167da9aec9ba1fbd8");
    assert_eq!(golden_b, "4e44cd9cf322c5ba2f01e7c285df1f10");
    assert_eq!(golden_c, "dfa62ed01072eef252fe81045b9aa5bc");
}

// ---------------------------------------------------------------------------
// RG-3: layer order — swapping tracks changes output exactly as specified
// ---------------------------------------------------------------------------

#[test]
fn rg3_layer_order() {
    let (input_ab, sources) = fixture_b(); // red bottom, green top
    let input_ba = RenderInput {
        tracks: input_ab.tracks.clone().into_iter().rev().collect(),
        output: input_ab.output.clone(),
    };
    let span = SPAN3;
    let frames_ab = render_all(&input_ab, span, &sources);
    let frames_ba = render_all(&input_ba, span, &sources);
    // frame 1: with green on top, (2,1) is green; with green at the bottom,
    // red covers the full frame and (2,1) is red.
    assert_eq!(px(&frames_ab[1], 2, 1), [0, 255, 0, 255]);
    assert_eq!(px(&frames_ba[1], 2, 1), [255, 0, 0, 255]);
    assert_ne!(golden_hash(&frames_ab), golden_hash(&frames_ba));
}

// ---------------------------------------------------------------------------
// RG-4: retime — 0.5x/2x render the exact source frames rational arithmetic
// predicts (through the ADR-013 seam, not a parallel path)
// ---------------------------------------------------------------------------

#[test]
fn rg4_retime_exact_frames() {
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1u64,
        Box::new(CountingSource {
            rate_num: 24,
            rate_den: 1,
            n: 64,
            tags: bt709(),
        }),
    );

    type SpeedCase = (Rational, fn(i64) -> i64);
    let cases: Vec<SpeedCase> = vec![
        (ticks(1, 2), |k: i64| (2 * k) / 5), // src_pts = k/60 s; idx = floor(k/60*24) = floor(2k/5)
        (ticks(2, 1), |k: i64| (8 * k) / 5), // src_pts = 2k/30 s; idx = floor(8k/5)
    ];
    for (speed, expected) in cases {
        let mut p = placement(
            1,
            ove_timeline::mapping::ClipWindow::retimed(
                ticks(0, 1),
                ticks(1, 1),
                ticks(0, 1),
                speed,
            ),
            1,
        );
        p.window = ove_timeline::mapping::ClipWindow::retimed(
            ticks(0, 1),
            ticks(1, 1),
            ticks(0, 1),
            speed,
        );
        let input = RenderInput {
            tracks: vec![TrackInput {
                placements: vec![p],
            }],
            output: output(8, 6, bt709()),
        };
        let frames = render_all(
            &input,
            RenderSpan {
                first: 0,
                count: 30,
            },
            &sources,
        );
        for (k, f) in frames.iter().enumerate() {
            let want = expected(k as i64);
            assert_eq!(
                px(f, 0, 0),
                [((want * 10) as u8), want as u8, 0, 255],
                "speed {speed} frame {k}: source frame {want} expected"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RG-5: split continuity — split at t renders identically to unsplit across
// the seam (the P7 invariant, end-to-end through the real timeline)
// ---------------------------------------------------------------------------

#[test]
fn rg5_split_continuity() {
    // Unsplit: one 1s clip.
    let mut tl = Timeline::new();
    tl.add_track(1, TrackKind::Gap(GapTrack::new())).unwrap();
    let id1 = tl.alloc_id();
    tl.apply(&Command::Insert {
        track: 1,
        index: 0,
        clip: Clip::new(id1, ticks(1, 1), ticks(0, 1)),
    })
    .unwrap();

    // Split at 0.5s: left keeps id1, right gets id2 (caller-allocated — E-012).
    let id2 = tl.alloc_id();
    tl.apply(&Command::Split {
        track: 1,
        id: id1,
        at: ticks(1, 2),
        new_id: id2,
    })
    .unwrap();

    // Walk the real timeline -> placements (the W5 engine path, exercised now).
    let mut placements = Vec::new();
    tl.track_ref(1).unwrap().walk(&mut |_pos, start, clip| {
        placements.push(placement(
            clip.id,
            ove_timeline::mapping::ClipWindow::new(start, clip.duration, clip.source_in),
            1,
        ));
    });
    assert_eq!(placements.len(), 2, "split produced two clips");

    let mut unsplit_placements = Vec::new();
    let mut tl2 = Timeline::new();
    tl2.add_track(1, TrackKind::Gap(GapTrack::new())).unwrap();
    let uid = tl2.alloc_id();
    tl2.apply(&Command::Insert {
        track: 1,
        index: 0,
        clip: Clip::new(uid, ticks(1, 1), ticks(0, 1)),
    })
    .unwrap();
    tl2.track_ref(1).unwrap().walk(&mut |_pos, start, clip| {
        unsplit_placements.push(placement(
            clip.id,
            ove_timeline::mapping::ClipWindow::new(start, clip.duration, clip.source_in),
            1,
        ));
    });

    let input_split = RenderInput {
        tracks: vec![TrackInput { placements }],
        output: output(8, 6, bt709()),
    };
    let input_unsplit = RenderInput {
        tracks: vec![TrackInput {
            placements: unsplit_placements,
        }],
        output: output(8, 6, bt709()),
    };

    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1u64,
        Box::new(CountingSource {
            rate_num: 24,
            rate_den: 1,
            n: 64,
            tags: bt709(),
        }),
    );
    // frames across the seam (0.5s = frame 15): render 0..30
    let span = RenderSpan {
        first: 0,
        count: 30,
    };
    let split_frames = render_all(&input_split, span, &sources);
    let unsplit_frames = render_all(&input_unsplit, span, &sources);
    for (k, (a, b)) in split_frames.iter().zip(unsplit_frames.iter()).enumerate() {
        assert_eq!(
            px(a, 0, 0),
            px(b, 0, 0),
            "split continuity broken at output frame {k}"
        );
    }
    assert_eq!(golden_hash(&split_frames), golden_hash(&unsplit_frames));
}

// ---------------------------------------------------------------------------
// RG-6: optimizer safety — culled placements provably do not change output
// ---------------------------------------------------------------------------

#[test]
fn rg6_optimizer_safety() {
    // Input X: real layer + three cullable placements (alpha 0, offset past
    // the right edge, offset past the bottom edge).
    let real = placement(
        1,
        ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
        1,
    );
    let mut culled = Vec::new();
    let mut invisible = placement(
        2,
        ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
        2,
    );
    invisible.alpha = Rational::zero(1);
    culled.push(invisible);
    let mut offscreen_r = placement(
        3,
        ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
        2,
    );
    offscreen_r.offset = (8, 0); // top-left past the right edge: provably invisible
    culled.push(offscreen_r);
    let mut offscreen_b = placement(
        4,
        ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
        2,
    );
    offscreen_b.offset = (0, 6); // top-left past the bottom edge
    culled.push(offscreen_b);

    let mut placements_x = vec![real];
    placements_x.extend(culled.iter().cloned());
    let input_x = RenderInput {
        tracks: vec![TrackInput {
            placements: placements_x,
        }],
        output: output(8, 6, bt709()),
    };
    let input_clean = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![placement(
                1,
                ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
                1,
            )],
        }],
        output: output(8, 6, bt709()),
    };

    // The cullable placements must produce NO passes (compile-level cull).
    let plan = compile_frame(&input_x, 0).unwrap();
    assert!(
        plan.passes.iter().all(|p| p.clip_id == 1),
        "cullable placements leaked passes into the plan"
    );
    // byte-identical output vs the clean input (execution-level safety)
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1u64,
        Box::new(SolidSource {
            color: [255, 0, 0],
            tags: bt709(),
        }),
    );
    sources.insert(
        2u64,
        Box::new(SolidSource {
            color: [0, 255, 0],
            tags: bt709(),
        }),
    );
    let fx = render_all(&input_x, SPAN3, &sources);
    let fc = render_all(&input_clean, SPAN3, &sources);
    assert_eq!(golden_hash(&fx), golden_hash(&fc));
}

// ---------------------------------------------------------------------------
// RG-7: color-tag propagation — convert ONCE, never twice, never silently
// ---------------------------------------------------------------------------

#[test]
fn rg7_color_tag_single_conversion() {
    // 601 source into 709 workspace: exactly ONE ColorConvert pass.
    let mut p601 = placement(
        1,
        ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
        1,
    );
    p601.src_color = bt601();
    let input = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![p601],
        }],
        output: output(8, 6, bt709()),
    };
    let plan = compile_frame(&input, 0).unwrap();
    let converts = plan
        .passes
        .iter()
        .filter(|p| matches!(p.kind, ove_render::PassKind::ColorConvert { .. }))
        .count();
    assert_eq!(converts, 1, "601->709 must convert exactly once");

    // 709 source into 709 workspace: ZERO convert passes.
    let input709 = RenderInput {
        tracks: vec![TrackInput {
            placements: vec![placement(
                1,
                ove_timeline::mapping::ClipWindow::new(ticks(0, 1), ticks(3, 30), ticks(0, 1)),
                1,
            )],
        }],
        output: output(8, 6, bt709()),
    };
    let plan709 = compile_frame(&input709, 0).unwrap();
    assert!(plan709
        .passes
        .iter()
        .all(|p| !matches!(p.kind, ove_render::PassKind::ColorConvert { .. })));

    // Execution: the 601-tagged frame goes through the convert pass and the
    // output is declared in the working space.
    let mut sources: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    sources.insert(
        1u64,
        Box::new(SolidSource {
            color: [128, 64, 32],
            tags: bt601(),
        }),
    );
    let frames = render_all(&input, SPAN3, &sources);
    assert_eq!(
        frames[0].color,
        bt709(),
        "output must be in the working space"
    );

    // Honesty: a frame whose tags differ from its declaration AND has no
    // convert pass must ERROR, never silently normalize.
    let mut bad: HashMap<u64, Box<dyn FrameSource>> = HashMap::new();
    bad.insert(
        1u64,
        Box::new(SolidSource {
            color: [1, 2, 3],
            tags: bt601(),
        }),
    );
    let plans = compile_span(&input709, SPAN3).unwrap();
    let renderer = SoftwareRenderer::new(bad.iter().map(|(k, v)| (*k, &**v)).collect());
    let err = renderer.execute_frame(&plans[0]).unwrap_err();
    assert!(matches!(err, RenderError::TagMismatch { .. }));
}
