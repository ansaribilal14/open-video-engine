# E-004b — UniFFI Kotlin bindings for the ove-time API (codegen + JVM runtime)

> Status: RUN-COMPLETE (codegen ✓, unmodified-Kotlin compile ✓, runtime roundtrip 8/8).

- **QUESTION**: Can the Rust core's exact-time API be exposed to Kotlin (Android shell)
  via UniFFI without hand-written JNI, and does the generated Kotlin actually compile
  and RUN correctly (exact values, no fp leakage)?
- **HYPOTHESIS**: proc-macro UniFFI over a small records/enums/functions surface works
  out of the box; the JVM desktop leg (JNA + Linux cdylib) is a valid pre-Android
  proving ground for the binding layer (E-004c remains the on-device JNI leg).
- **IMPLEMENTATION**:
  - `engine/ove-time` — the real crate under test (exact `Rational`, i128-guarded ops,
    `floor_div_rate`), dependency-free.
  - `scripts/experiments/E-004b_uniffi/` — FFI wrapper crate `ove-time-ffi`
    (`#[derive(uniffi::Record)] RationalTime`, `#[derive(uniffi::Enum)] TimeValue`
    with `TickAxis`/`FreeRational` variants, 6 exported functions, `Vec<RationalTime>`
    return). uniffi 0.32.2, rustc 1.98.1.
  - Codegen: `uniffi-bindgen generate --library libove_time_ffi.so --language kotlin`.
  - Compile check: kotlinc 2.1.20 (JRE 21) + JNA 5.14.0.
  - Runtime check: `E004bTest.kt` main() against the Linux cdylib on the desktop JVM
    (`-Djna.library.path=target/debug`).
- **HARDWARE**: container CPU (correctness only; 1 fs timing irrelevant here).
- **RESULT**:
  - Codegen: 1,283-line `ove_time_ffi.kt`; records → data classes, enums → sealed
    classes, sequences → `List<T>`; JNA is the only external dependency; helper runtime
    bundled inline by the generator. ktlint auto-format skipped (not installed) —
    cosmetic only.
  - Compile: **unmodified generated Kotlin compiles** under kotlinc 2.1.20.
  - Runtime (`experiments/E-004b_result.txt`): **8/8 PASS** —
    R1 exact add across FFI (1001/24000 + 1001/24000 → 1001/12000 normalized);
    R2 add/sub roundtrip; R3 split roundtrip (half + remainder == original);
    R4/R4b boundary floor exact where fp fails (1s @30fps → 30, 29999/30000 → 29);
    R5 tick-axis rescale exact (4800 ticks @24000/1001 → 9,619,200 ticks @48kHz);
    R6 FreeRational variant agrees; R7 API surface contains **no** Double/Float
    parameters (verified by reflection) — the fp ban is structural, not conventional.
- **LIMITATIONS**: desktop JVM/JNA leg, not Android ART/JNI (E-004c: thread attach,
  exception translation, callback marshalling on-device remain open); perf not
  measured (debug cdylib); async/objects/callbacks surface untested (time-only API).
- **DECISION**: ADR-001 evidence strengthened — the "UniFFI codegen/ergonomics gaps"
  revisit trigger is cleared for the domain-data portion of the API. The ove-time
  boundary (i64 num/den records, no fp) is now proven end-to-end Rust→Kotlin. The
  command/object/callback surface still needs its own binding slice before Phase 2.
