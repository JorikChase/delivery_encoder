use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use std::sync::mpsc;
use std::thread;
use std::fs;
use std::io::{BufRead, BufReader};

// Helper function to get number of available threads
fn get_available_threads() -> usize {
    match std::thread::available_parallelism() {
        Ok(n) => {
            let threads = n.get();
            println!("🧵 System reports {} available threads", threads);
            threads
        }
        Err(e) => {
            println!("⚠️ Failed to get thread count: {}, using 1 thread", e);
            1
        }
    }
}

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

    // Create temporary segments directory
    let segments_dir = "tmp_segments";
    println!("\n📂 Creating temporary segments directory: {}", segments_dir);
    if Path::new(segments_dir).exists() {
        println!("⚠️ Temporary directory exists, cleaning...");
        if let Err(e) = fs::remove_dir_all(segments_dir) {
            println!("❌ Failed to clean existing segments directory: {}", e);
            std::process::exit(1);
        }
    }
    if let Err(e) = fs::create_dir(segments_dir) {
        println!("❌ Failed to create segments directory: {}", e);
        std::process::exit(1);
    }
    println!("✅ Created temporary segments directory");

    // Get video duration using FFprobe
    let ffprobe_path = if cfg!(windows) {
        ffmpeg_path.replace("ffmpeg.exe", "ffprobe.exe")
    } else {
        ffmpeg_path.replace("ffmpeg", "ffprobe")
    };

    println!("\n⏱ Measuring video duration with FFprobe...");
    println!("🔍 FFprobe path: {}", ffprobe_path);

    let duration_output = Command::new(&ffprobe_path)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            "assets/video.mov"
        ])
        .output()
        .unwrap_or_else(|e| {
            println!("❌ Failed to execute ffprobe: {}", e);
            std::process::exit(1);
        });

    if !duration_output.status.success() {
        let error_msg = String::from_utf8_lossy(&duration_output.stderr);
        println!("❌ FFprobe failed: {}", error_msg);
        std::process::exit(1);
    }

    let duration_str = String::from_utf8_lossy(&duration_output.stdout);
    let total_duration: f64 = duration_str.trim().parse().unwrap_or_else(|_| {
        println!("❌ Failed to parse video duration: '{}'", duration_str);
        std::process::exit(1);
    });

    println!("⏱ Total video duration: {:.2} seconds", total_duration);

    // Determine number of threads to use
    let num_threads = get_available_threads().max(1);
    println!("🧵 Using {} threads for parallel processing", num_threads);

    // Calculate segment duration
    let segment_duration = total_duration / num_threads as f64;
    println!("⏱ Segment duration: {:.2} seconds", segment_duration);

    // Create channel for thread communication
    let (tx, rx) = mpsc::channel();

    println!("\n⚙️ Starting parallel processing...");
    let processing_start = Instant::now();

    // Spawn worker threads
    for thread_id in 0..num_threads {
        let tx = tx.clone();
        let ffmpeg_path = ffmpeg_path.to_string();
        let segments_dir = segments_dir.to_string();
        
        println!("🧵 Starting thread {} for segment {}...", thread_id, thread_id);
        
        thread::spawn(move || {
            let start_time = thread_id as f64 * segment_duration;
            let segment_dir = format!("{}/segment_{}", segments_dir, thread_id);
            
            // Create segment-specific directory
            if let Err(e) = fs::create_dir(&segment_dir) {
                println!("❌ [Thread {}] Failed to create segment directory: {}", thread_id, e);
                tx.send((thread_id, false)).unwrap();
                return;
            }
            
            let output_pattern = format!("{}/%05d.png", segment_dir);
            
            let args = [
                "-ss", &start_time.to_string(),
                "-i", "assets/video.mov",
                "-i", "assets/overlay.png",
                "-filter_complex", "[0:v][1:v]overlay",
                "-t", &segment_duration.to_string(),
                "-y", &output_pattern
            ];

            println!("[Thread {}] Starting FFmpeg at {:.2}s for {:.2}s", 
                thread_id, start_time, segment_duration);
            println!("[Thread {}] Command: {} {}", 
                thread_id, ffmpeg_path, args.join(" "));

            let mut cmd = match Command::new(&ffmpeg_path)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn() 
            {
                Ok(cmd) => cmd,
                Err(e) => {
                    println!("❌ [Thread {}] Failed to spawn FFmpeg: {}", thread_id, e);
                    tx.send((thread_id, false)).unwrap();
                    return;
                }
            };

            // Capture and log stderr
            let stderr = cmd.stderr.take().unwrap();
            let reader = BufReader::new(stderr);
            let mut last_log_time = Instant::now();
            
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        // Log every 5 seconds or if there's an error
                        if line.contains("error") || line.contains("fail") || 
                           last_log_time.elapsed().as_secs() >= 5 {
                            println!("[Thread {}] {}", thread_id, line);
                            last_log_time = Instant::now();
                        }
                    }
                    Err(e) => {
                        println!("⚠️ [Thread {}] Error reading FFmpeg output: {}", thread_id, e);
                        break;
                    }
                }
            }

            let status = match cmd.wait() {
                Ok(status) => status,
                Err(e) => {
                    println!("❌ [Thread {}] Failed to wait for FFmpeg: {}", thread_id, e);
                    tx.send((thread_id, false)).unwrap();
                    return;
                }
            };

            if status.success() {
                println!("✅ [Thread {}] FFmpeg completed successfully", thread_id);
                tx.send((thread_id, true)).unwrap();
            } else {
                let exit_code = status.code().unwrap_or(-1);
                println!("❌ [Thread {}] FFmpeg failed with exit code: {}", thread_id, exit_code);
                tx.send((thread_id, false)).unwrap();
            }
        });
    }

    // Drop the original transmitter so the channel closes properly
    drop(tx);

    println!("⏳ Waiting for threads to complete...");

    // Collect results from worker threads
    let mut success_count = 0;
    for (i, (thread_id, success)) in rx.iter().enumerate() {
        if success {
            println!("✅ Thread {} completed successfully ({}/{})", 
                thread_id, i+1, num_threads);
            success_count += 1;
        } else {
            println!("❌ Thread {} failed ({}/{})", thread_id, i+1, num_threads);
        }
    }

    if success_count != num_threads {
        println!("❌ Only {}/{} threads completed successfully", success_count, num_threads);
        std::process::exit(1);
    }

    let processing_duration = processing_start.elapsed();
    println!("\n✅ Parallel processing completed in {:.2} seconds", processing_duration.as_secs_f32());

    // Combine processed segments
    println!("\n🔗 Combining segments...");
    let combine_start = Instant::now();
    let mut frame_counter = 1;

    for thread_id in 0..num_threads {
        let segment_path = format!("{}/segment_{}", segments_dir, thread_id);
        println!("🔍 Processing segment {}: {}", thread_id, segment_path);
        
        let segment_dir = Path::new(&segment_path);
        
        let entries = match fs::read_dir(segment_dir) {
            Ok(entries) => entries,
            Err(e) => {
                println!("❌ Error reading segment {} directory: {}", thread_id, e);
                continue;
            }
        };
        
        let mut frames: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().map_or(false, |ext| ext == "png"))
            .collect();
        
        if frames.is_empty() {
            println!("⚠️ No PNG frames found in segment {}: {}", thread_id, segment_path);
            continue;
        }
        
        // Sort frames numerically
        frames.sort_by(|a, b| {
            a.file_name()
                .and_then(|na| na.to_str())
                .and_then(|na| na.split('.').next())
                .and_then(|na| na.parse::<u32>().ok())
                .cmp(
                    &b.file_name()
                        .and_then(|nb| nb.to_str())
                        .and_then(|nb| nb.split('.').next())
                        .and_then(|nb| nb.parse::<u32>().ok())
                )
        });

        println!("📦 Segment {} has {} frames", thread_id, frames.len());
        
        for frame in frames {
            let new_name = format!("video{:05}.png", frame_counter);
            let dest = Path::new(output_dir).join(new_name);
            
            if let Err(e) = fs::rename(&frame, &dest) {
                println!("❌ Error moving file {}: {}", frame.display(), e);
            }
            
            frame_counter += 1;
        }
    }

    let combine_duration = combine_start.elapsed();
    println!("✅ Combined {} frames in {:.2} seconds", frame_counter - 1, combine_duration.as_secs_f32());

    // Clean up temporary directory
    println!("\n🧹 Cleaning up temporary files...");
    if let Err(e) = fs::remove_dir_all(segments_dir) {
        println!("⚠️ Failed to clean temporary directory: {}", e);
    } else {
        println!("✅ Temporary files cleaned");
    }

    // Final statistics
    let total_duration = start_time.elapsed();
    println!("\n🏁 Total execution time: {:.2} seconds\n✨ Process completed", 
        total_duration.as_secs_f32()
    );
}