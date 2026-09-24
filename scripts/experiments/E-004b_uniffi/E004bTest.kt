// E-004b runtime leg: exercise the generated Kotlin bindings against the
// Rust cdylib on a desktop JVM (JNA). Passes only if values round-trip
// EXACTLY (no float anywhere on the path).
import uniffi.ove_time_ffi.*

fun main() {
    var passed = 0
    var failed = 0
    fun check(name: String, cond: () -> Boolean) {
        val ok = try { cond() } catch (t: Throwable) { println("  (threw: $t)"); false }
        if (ok) { passed++ ; println("PASS $name") } else { failed++ ; println("FAIL $name") }
    }

    // R1: exact add across the FFI: 1001/24000 + 1001/24000 = 2002/24000 -> normalized 1001/12000
    val frame = RationalTime(num = 1001, den = 24000)
    val two = oveTimeAdd(frame, frame)
    check("R1 exact add") { two.num == 1001L && two.den == 12000L }

    // R2: exact sub roundtrip: (a + b) - b == a
    val a = RationalTime(num = 123_457, den = 24000)
    val b = RationalTime(num = -987, den = 48000)
    val rt = oveTimeSub(oveTimeAdd(a, b), b)
    check("R2 add/sub roundtrip") { rt.num == a.num && rt.den == a.den }

    // R3: half + remainder == original (split invariant)
    val halves = oveTimeSplitRoundtrip(a)
    val rejoined = oveTimeAdd(halves[0], halves[1])
    check("R3 split roundtrip") { rejoined.num == a.num && rejoined.den == a.den }

    // R4: boundary floor where fp fails: exactly 30 frames at 30fps -> 30 (fp got 29)
    val oneSec = RationalTime(num = 1, den = 1)
    check("R4 boundary floor exact") { oveTimeFloorFrame(oneSec, 30, 1) == 30L }
    check("R4b just below boundary") {
        oveTimeFloorFrame(RationalTime(num = 29_999, den = 30_000), 30, 1) == 29L
    }

    // R5: enum TickAxis -> ticks on another axis, exact rescale
    // 4800 ticks at 24000/1001 == how many ticks at 48000 Hz? 2 * 4800 = 9600? (24000/1001 s per tick)
    val v = TimeValue.TickAxis(ticks = 4800, rateNum = 24000, rateDen = 1001)
    val t48000 = oveTimeToTicks(v, 48000, 1)
    val expected48000 = oveTimeFloorFrame(RationalTime(num = 4800 * 1001, den = 24000), 48000, 1)
    check("R5 tick-axis rescale") { t48000 == expected48000 && t48000 == 4800L * 1001 * 2 }

    // R6: FreeRational variant reaches the same exact answer
    val fr = TimeValue.FreeRational(num = 4800 * 1001, den = 24000)
    check("R6 free rational -> ticks") { oveTimeToTicks(fr, 48000, 1) == expected48000 }

    // R7: 23.976 alias must NOT be accepted as authoritative — API shape has no
    //     fp constructor at all (structural check): no exported function takes Double/Float.
    check("R7 no fp on API surface") {
        val facade = Class.forName("uniffi.ove_time_ffi.Ove_time_ffiKt")
        facade.declaredMethods.none { m ->
            m.parameterTypes.any { it == java.lang.Double.TYPE || it == java.lang.Float.TYPE }
        }
    }

    println("E-004b runtime leg: $passed passed, $failed failed")
    if (failed > 0) kotlin.system.exitProcess(1)
}
