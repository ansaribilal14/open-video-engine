//! Example: the feature-detect + fallback contract (BUILD_PLAN wave 6).
//! Without a usable adapter `GpuRenderer::new` fails TYPED (NoAdapter) —
//! the documented trigger for constructing a SoftwareRenderer instead.
//! Run WITHOUT a Vulkan ICD to see the fallback path; with one (lavapipe)
//! you get the adapter identity instead.

#[cfg(feature = "gpu")]
fn main() {
    match ove_render::GpuRenderer::new() {
        Ok(gpu) => println!("GPU-ADAPTER-OK: {}", gpu.adapter_summary()),
        Err(ove_render::GpuError::NoAdapter { detail }) => {
            println!("TYPED-NO-ADAPTER-OK: {detail}");
        }
        Err(e) => println!("TYPED-GPU-ERROR: {e}"),
    }
}

#[cfg(not(feature = "gpu"))]
fn main() {
    println!("build with --features gpu");
}
