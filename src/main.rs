
use std::env;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

fn main() {
    let start_time = Instant::now();
    println!("🚀 Starting delivery encoder\n---------------------------");

    // Get executable path and derive project root
    let exe_path = env::current_exe().unwrap_or_else(|e| {
        println!("❌ Failed to get executable path: {}", e);
        std::process::exit(1);
    });
    println!("✅ Executable path: {}", exe_path.display());

    let project_root = exe_path
        .parent()  // bin/<os>
        .and_then(|p| p.parent())  // bin
        .and_then(|p| p.parent())  // project root
        .unwrap_or_else(|| {
            println!("❌ Failed to derive project root");
            std::process::exit(1);
        });

    println!("📂 Project root: {}", project_root.display());

    // Set working directory
    if let Err(e) = env::set_current_dir(project_root) {
        println!("❌ Failed to set working directory: {}", e);
        std::process::exit(1);
    }
    println!("📂 Working directory set to project root");

    // Determine FFmpeg path
    let ffmpeg_path = match () {
        _ if cfg!(target_os = "macos") => "assets/bin/macos/ffmpeg",
        _ if cfg!(target_os = "windows") => "assets/bin/windows/ffmpeg.exe",
        _ => {
            println!("❌ Unsupported operating system");
            std::process::exit(1);
        }
    };

    println!("🔍 FFmpeg path: {}\n✅ Platform: {}", 
        ffmpeg_path,
        if cfg!(windows) { "Windows" } else { "macOS" }
    );

    // Define and validate paths
    let assets = [
        ("Video", "assets/video.mov"),
        ("Overlay", "assets/overlay.png"),
        ("FFmpeg", ffmpeg_path),
    ];

    println!("\n🔍 Validating input files:");
    for (name, path) in &assets {
        let exists = Path::new(path).exists();
        println!("- {}: {} -> {}", name, path, exists);
        if !exists {
            println!("❌ {} not found: {}", name, path);
            std::process::exit(1);
        }
    }

    // Create output directory
    let output_dir = "output";
    println!("\n📂 Creating output directory: {}", output_dir);
    if !Path::new(output_dir).exists() {
        if let Err(e) = std::fs::create_dir(output_dir) {
            println!("❌ Failed to create output directory: {}", e);
            std::process::exit(1);
        }
        println!("✅ Created output directory");
    } else {
        println!("ℹ️ Output directory already exists");
    }

    // Prepare FFmpeg command
    let output_pattern = format!("{}/video%05d.png", output_dir);
    let args = [
        "-i", "assets/video.mov",
        "-i", "assets/overlay.png",
        "-filter_complex", "[0:v][1:v]overlay",
        "-y", &output_pattern
    ];

    println!("\n⚙️ FFmpeg command:\n{} {}", 
        ffmpeg_path,
        args.join(" ")
    );

    println!("\n⏳ Starting video processing...");
    let ffmpeg_start = Instant::now();

    // Execute FFmpeg command
    let status = Command::new(ffmpeg_path)
        .args(&args)
        .status();

    // Handle execution result
    match status {
        Ok(exit_status) if exit_status.success() => {
            let duration = ffmpeg_start.elapsed();
            println!("\n✅ Conversion successful!");
            println!("⏱️ FFmpeg processing time: {:.2} seconds", duration.as_secs_f32());
            println!("📸 PNG frames saved to: {}", output_pattern);
        },
        Ok(exit_status) => {
            println!("\n❌ FFmpeg failed with exit code: {:?}", exit_status.code());
        },
        Err(e) => {
            println!("\n❌ Failed to execute FFmpeg command: {}", e);
        }
    }

    // Final statistics
    let total_duration = start_time.elapsed();
    println!("\n🏁 Total execution time: {:.2} seconds\n✨ Process completed", 
        total_duration.as_secs_f32()
    );
}