// E-004a: Rust FFI boundary experiment — the pattern the JNI/UniFFI layer must follow.
// Builds a cdylib exposing: exact-rational time transfer (i64 num/den), opaque handle
// lifecycle, error-code returns (no panics across FFI), and panic-catching boundary.
use std::os::raw::{c_int, c_char};

#[repr(C)]
pub struct OveTime { pub num: i64, pub den: i64 }

#[repr(C)]
pub struct OveClip { pub track: c_int, pub start: OveTime, pub dur: OveTime }

// error codes: 0 OK, 1 null, 2 invalid den, 3 caught panic
const OK: c_int = 0;

#[no_mangle]
pub extern "C" fn ove_clip_new(track: c_int, sn: i64, sd: i64, dn: i64, dd: i64) -> *mut OveClip {
    if sd == 0 || dd == 0 { return std::ptr::null_mut(); }
    Box::into_raw(Box::new(OveClip { track, start: OveTime { num: sn, den: sd }, dur: OveTime { num: dn, den: dd } }))
}

#[no_mangle]
pub unsafe extern "C" fn ove_clip_end(clip: *mut OveClip) -> OveTime {
    if clip.is_null() { return OveTime { num: -1, den: 1 }; }
    let c = &*clip;
    // exact end = start + dur (lcm math, same as bench)
    let g = gcd(c.start.den.abs(), c.dur.den.abs());
    if g == 0 { return OveTime { num: -1, den: 1 }; }
    let lcm = c.start.den / g * c.dur.den;
    OveTime { num: c.start.num * (lcm / c.start.den) + c.dur.num * (lcm / c.dur.den), den: lcm }
}

#[no_mangle]
pub unsafe extern "C" fn ove_clip_free(clip: *mut OveClip) {
    if !clip.is_null() { drop(Box::from_raw(clip)); }
}

// panic-catching boundary: panics must NEVER cross FFI (UB in JNI callers)
#[no_mangle]
pub extern "C" fn ove_risky(op: c_int) -> c_int {
    let result = std::panic::catch_unwind(|| match op {
        0 => OK,
        1 => panic!("intentional internal panic"),
        _ => -1,
    });
    match result { Ok(code) => code, Err(_) => 3 }
}

fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a } else { gcd(b, a % b) } }

#[no_mangle]
pub extern "C" fn ove_version() -> *const c_char { b"0.1.0-e004a\0".as_ptr() as *const c_char }
