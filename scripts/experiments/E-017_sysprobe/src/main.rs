// W2 probe: does ffmpeg-sys-next (bundled) build + link in this sandbox?
// Success = prints libav library versions; failure modes recorded honestly.
fn main() {
    unsafe {
        println!("libavformat = {}", ffmpeg_sys_next::avformat_version());
        println!("libavcodec  = {}", ffmpeg_sys_next::avcodec_version());
        println!("libavutil   = {}", ffmpeg_sys_next::avutil_version());
        println!("av_version_info = {}", {
            let p = ffmpeg_sys_next::av_version_info();
            std::ffi::CStr::from_ptr(p).to_string_lossy()
        });
    }
}
