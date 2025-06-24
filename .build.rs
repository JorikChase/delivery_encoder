use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=bin");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let platform_dir = match target_os.as_str() {
        "windows" => "windows",
        "macos" => "macos",
        _ => panic!("Unsupported OS: {}", target_os),
    };

    // Set paths relative to CARGO_MANIFEST_DIR
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Set asset paths
    let video_path = Path::new(&manifest_dir).join("assets/video.prores");
    println!("cargo:rustc-env=VIDEO_PATH={}", video_path.display());

    let overlay_path = Path::new(&manifest_dir).join("assets/overlay.png");
    println!("cargo:rustc-env=OVERLAY_PATH={}", overlay_path.display());

    // Set FFmpeg binary paths
    let ffmpeg_path = Path::new(&manifest_dir).join(format!(
        "bin/{}/ffmpeg{}",
        platform_dir,
        if target_os == "windows" { ".exe" } else { "" }
    ));
    println!("cargo:rustc-env=FFMPEG_PATH={}", ffmpeg_path.display());

    let ffprobe_path = Path::new(&manifest_dir).join(format!(
        "bin/{}/ffprobe{}",
        platform_dir,
        if target_os == "windows" { ".exe" } else { "" }
    ));
    println!("cargo:rustc-env=FFPROBE_PATH={}", ffprobe_path.display());
}
