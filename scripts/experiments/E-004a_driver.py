#!/usr/bin/env python3
"""E-004a driver: exercises the Rust FFI boundary via ctypes (stands in for the JNI proxy
layer in this container: same ABI questions — exact i64 time transfer, opaque handles,
error codes, panic containment)."""
import ctypes, sys, os, json

lib_path = sys.argv[1]
lib = ctypes.CDLL(lib_path)

class OveTime(ctypes.Structure):
    _fields_ = [("num", ctypes.c_int64), ("den", ctypes.c_int64)]

class OveClip(ctypes.Structure):
    _fields_ = [("track", ctypes.c_int), ("start", OveTime), ("dur", OveTime)]

lib.ove_clip_new.restype = ctypes.c_void_p
lib.ove_clip_new.argtypes = [ctypes.c_int, ctypes.c_int64, ctypes.c_int64, ctypes.c_int64, ctypes.c_int64]
lib.ove_clip_end.restype = OveTime
lib.ove_clip_end.argtypes = [ctypes.c_void_p]
lib.ove_clip_free.argtypes = [ctypes.c_void_p]
lib.ove_risky.restype = ctypes.c_int
lib.ove_risky.argtypes = [ctypes.c_int]

res = []
def check(name, ok, detail):
    res.append((name, bool(ok), detail)); print(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}")

# 1. opaque handle lifecycle + exact rational end-time across boundary
h = lib.ove_clip_new(1, 1001, 24000, 48000 * 1001, 30000)   # NTSC clip, 1601.6 samples/frame dur? dur=48000*1001/30000 ticks
end = lib.ove_clip_end(h)
# exact expected: 1001/24000 + 48000*1001/30000 = 1001/24000 + 8*1001/5 = (1001*5 + 8*1001*48000)/(5*24000)
from fractions import Fraction as Fr
expected = Fr(1001, 24000) + Fr(48000 * 1001, 30000)
got = Fr(end.num, end.den) if end.den != 0 else None
check("handle + exact rational end", end.den != 0 and got == expected, f"got {end.num}/{end.den}, expected {expected}")

# 2. free without crash (use-after-free guard is Rust's job)
lib.ove_clip_free(h)
check("free completes", True, "opaque handle dropped in Rust")

# 3. null safety
end_null = lib.ove_clip_end(None)
check("null handle -> sentinel", end_null.num == -1, f"num={end_null.num}")

# 4. invalid den rejected at construction
h_bad = lib.ove_clip_new(0, 1, 0, 1, 1)
check("zero-den rejected (null)", h_bad in (0, None), f"ptr={h_bad}")

# 5. panic containment at boundary
code_ok = lib.ove_risky(0)
code_panic = lib.ove_risky(1)
check("panic contained (process alive, code 3)", code_ok == 0 and code_panic == 3, f"op0={code_ok}, op1={code_panic}")

fails = [r for r in res if not r[1]]
print("\n==== E-004a SUMMARY ====")
print(f"checks: {len(res)}, failed: {len(fails)}")
verdict = "FFI boundary pattern viable: exact i64 time + opaque handles + error codes + catch_unwind" if not fails else "investigate"
print("VERDICT:", verdict)
out = os.path.join(os.path.dirname(__file__), "../../experiments/E-004a_result.txt")
with open(out, "w") as f:
    for name, ok, detail in res: f.write(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}\n")
    f.write(f"VERDICT: {verdict}\n")
