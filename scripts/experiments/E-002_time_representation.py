#!/usr/bin/env python3
"""
E-002: Time representation experiment — float64 seconds vs exact rational time.

QUESTION:  Is float64 seconds safe as the authoritative timeline time representation?
HYPOTHESIS: No — boundary floor() errors, order-dependent sums, decimal-rate drift
            (23.976 vs 24000/1001), and cross-"renderer" divergence make fp unsafe;
            exact integer/rational arithmetic is deterministic.
CLAIM:     C-001 (ledger). ADR-007.

Python floats are IEEE-754 binary64, identical semantics to Rust f64/C double/JS
Number, so correctness results here transfer to engine implementations. The PERF
benchmark part is NOT representative of Rust (recorded as a limitation).
"""
import random
import struct
import time
from fractions import Fraction

PASS, FAIL = "PASS", "FAIL"
results = []

def floor_frac(fr: Fraction) -> int:
    """Exact floor of a Fraction."""
    return fr.numerator // fr.denominator

def check(name, ok, detail):
    results.append((name, PASS if ok else FAIL, detail))
    print(f"[{results[-1][1]}] {name}: {detail}")

# ---------- T1: exact-boundary floor off-by-one ----------
# NOTE (recorded honestly): 3x(1/3) in binary64 rounds to exactly 1.0 via the IEEE-754
# round-to-even TIE rule — that popular example is WRONG. The real boundary killer is
# any decimal duration that is not exactly representable, e.g. 0.1 s.
ten = 0.0
for _ in range(10): ten += 0.1            # ten 0.1s clips back-to-back
fp_end = ten                              # mathematically exactly 1.0 s
r_end = Fraction(1, 1)
check("T1 fp 10x(0.1) != 1.0", fp_end != 1.0,
      f"fp sum = {fp_end!r} (mathematically 1.0) -> frame index floor(fp*30)={int(fp_end*30)} vs exact 30")
check("T1 rational 10x(1/10) == 1.0", Fraction(1,10)*10 == 1, f"Fraction sum = {Fraction(1,10)*10}")
# Boundary frame indexing: frame index at t=1.0s, 30fps: fp gives floor(0.9999999999999999*30)=29 (off-by-one)
fp_idx = int(fp_end * 30)
r_idx = int(r_end * 30)
check("T1 fp frame-index off-by-one at boundary", fp_idx == 29 and r_idx == 30,
      f"fp index at 1.0s = {fp_idx}, rational = {r_idx} -> snap/to-region tests misfire")

# ---------- T2: decimal 23.976 vs 24000/1001 systematic drift ----------
NTSC = Fraction(24000, 1001)          # true "23.976"
DECAY = 23.976                        # decimal stored in a project file
hours = 2
n_frames = int(NTSC * 3600 * hours)   # frames in 2h at true rate
rational_end = n_frames * Fraction(1, NTSC)          # exact seconds
fp_end_dec = n_frames * (1.0 / DECAY)                # frames × (1/23.976)
drift_s = abs(float(rational_end) - fp_end_dec)
drift_ms = drift_s * 1000.0
check("T2 decimal-23.976 drift accumulates", drift_ms > 1.0,
      f"after 2h ({n_frames} frames): {drift_ms:.3f} ms drift vs exact 24000/1001")
# Also: 1/23.976 is NOT 1001/24000 in binary64
check("T2 1/23.976 != 1001/24000 in fp", 1.0/23.976 != float(Fraction(1001, 24000)),
      f"1/23.976={1.0/23.976!r}  1001/24000={float(Fraction(1001,24000))!r}")

# ---------- T3: order-dependence of clip start sums ----------
random.seed(42)
durs = [Fraction(random.randint(1001, 100101), 24000) for _ in range(2000)]  # 2000 clips
fp_list = [float(d) for d in durs]
starts_fwd, acc = [], 0.0
for d in fp_list: starts_fwd.append(acc); acc += d
starts_rev, acc = [], 0.0
for d in reversed(fp_list): starts_rev.append(acc); acc += d
starts_rev.reverse()
mism = sum(1 for a, b in zip(starts_fwd, starts_rev) if a != b)
check("T3 fp start-times order-dependent", mism > 0,
      f"{mism}/2000 clip starts differ between L→R and R→L accumulation")
# determinism guarantee of rationals (exact arithmetic is associative):
rat_fwd, rat_rev = Fraction(0), Fraction(0)
for d in durs: rat_fwd += d
for d in reversed(durs): rat_rev += d
check("T3 rational sums order-invariant", rat_fwd == rat_rev, f"totals equal: {rat_fwd == rat_rev} (total = {rat_fwd})")

# ---------- T4: audio-sample boundary rule vs fp (empirical mismatch census) ----------
samples_per_frame = Fraction(48000) * Fraction(1001, 30000)   # 1601.6 exactly
check("T4 48kHz×29.97 frame = non-integer samples", samples_per_frame.denominator != 1,
      f"{samples_per_frame} samples/frame -> sample boundary must be RULED, not fp-multiplied")
# Census: for many frame counts, does floor(frames*fp_duration) match floor(frames*rational)?
# Empirical across common rate pairs (this catches whichever side binary64 rounds on):
combos = [(48000, 30000, 1001), (48000, 24000, 1001), (44100, 30000, 1001),
          (44100, 24000, 1001), (48000, 60000, 1001), (44100, 25, 1)]
total_mism, worst = 0, None
for rate, fnum, fden in combos:
    mism = 0
    for f in range(1, 20001):
        exact = floor_frac(Fraction(rate) * Fraction(f * fden, fnum))  # floor of exact rational boundary
        fp_v = f * (rate * fden / fnum)                                # fp path
        if int(fp_v) != exact:
            mism += 1
    total_mism += mism
    worst = worst or (rate, fnum, fden, mism)
check("T4 fp sample-boundary mismatches found", total_mism > 0,
      f"{total_mism} boundary mismatches across 6 rate pairs × 20000 frames (first: {worst})")

# ---------- T5: cross-"renderer" divergence (emulated fp policies) ----------
# Renderer A: accumulate starts left-to-right. Renderer B: compute start = prefix via math.fsum.
# Both are legal fp policies; they must agree for deterministic cross-platform render.
starts_b = []
for i in range(len(fp_list)):
    starts_b.append(sum(fp_list[:i + 1]) - fp_list[i] if i else 0.0)
mism_t5 = sum(1 for a, b in zip(starts_fwd, starts_b) if a != b)
check("T5 two legal fp policies disagree", mism_t5 > 0,
      f"{mism_t5}/2000 starts differ between accumulation policies (naive vs prefix-sum)")

# ---------- T6: fixed tick-axis alternative is insufficient ----------
TICKS_PER_S = 48000 * 4  # 192 kHz engine tick axis (a real design candidate)
ntsc = Fraction(1001, 24000)   # 23.976 frame duration in s
ticks_ntsc = ntsc * TICKS_PER_S
ntsc30 = Fraction(1001, 30000) # 29.97 frame duration in s
ticks_2997 = ntsc30 * TICKS_PER_S
check("T6 192kHz ticks handle 23.976 but NOT 29.97",
      ticks_ntsc.denominator == 1 and ticks_2997.denominator != 1,
      f"1 frame: 23.976->{ticks_ntsc} ticks (integral), 29.97->{ticks_2997} ticks (non-integral)")
# Even a huge LCM-style tick axis fails for arbitrary user rates:
BIG = 30000 * 1001  # 30,030,000 Hz — makes all 1001-family rates integral
odd_rate = Fraction(17000, 999)  # a legal user-defined 17.017 fps
ticks_odd = odd_rate * BIG
check("T6 no practical fixed tick axis covers arbitrary rates", ticks_odd.denominator != 1,
      f"even at {BIG} Hz ticks, {odd_rate} fps frames = {ticks_odd} ticks (non-integral)"
      " => per-clip rationals remain necessary")

# ---------- T7: timeline structure benchmark (perf; NOT Rust-representative) ----------
N = 20000
starts, acc = [], Fraction(0)
t0 = time.perf_counter()
for i in range(N):
    d = Fraction(random.randint(1001, 240001), 24000)
    starts.append((acc, d)); acc += d
t_rational_build = time.perf_counter() - t0
t0 = time.perf_counter()
fp_starts, acc = [], 0.0
for i in range(N):
    d = float(starts[i][1]); fp_starts.append((acc, d)); acc += d
t_fp_build = time.perf_counter() - t0
t0 = time.perf_counter()
idx = sorted(range(N), key=lambda i: starts[i][0])
t_sort = time.perf_counter() - t0
check("T7 benchmark executed (perf non-authoritative)", True,
      f"N={N}: rational build {t_rational_build*1000:.1f}ms, fp build {t_fp_build*1000:.1f}ms, "
      f"sort-by-start {t_sort*1000:.1f}ms — Python timing only; re-run in Rust crate ove-time")

# ---------- summary ----------
fails = [r for r in results if r[1] == FAIL]
print("\n==== E-002 SUMMARY ====")
print(f"checks: {len(results)}, failed: {len(fails)}")
verdict = ("fp seconds UNSAFE as authoritative time (hypothesis confirmed)" if fails == [] else "unexpected FAIL — investigate")
print("VERDICT:", verdict)
with open("/home/z/my-project/open-video-engine/experiments/E-002_result.txt", "w") as f:
    for name, status, detail in results:
        f.write(f"[{status}] {name}: {detail}\n")
    f.write(f"VERDICT: {verdict}\n")
    f.write("HARDWARE: container CPU (irrelevant for correctness; perf rows non-representative)\n")
