use anyhow::{Context, Result};
use hound;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use tempfile::TempDir;
use tokio::task;
use transcribe_rs::TranscriptionSegment;

const CHUNK_DURATION: u32 = 300; // 5 minutes
const OVERLAP_DURATION: u32 = 10; // 10 seconds
const NUM_WORKERS: usize = 6; // Optimized for 24GB RAM & ~900MB Model

#[derive(Serialize)]
struct WorkerRequest {
    chunk_path: String,
}

#[derive(Deserialize)]
struct WorkerResponse {
    success: bool,
    segments: Option<Vec<TranscriptionSegment>>,
    error: Option<String>,
}

struct WorkerProcess {
    child: Child,
}

impl WorkerProcess {
    fn new(model_path: &Path) -> Result<Self> {
        let exe_path = std::env::current_exe()
            .context("Failed to get current exe path")?
            .parent()
            .context("Failed to get parent dir")?
            .to_path_buf();

        // 1. Try "transcribe-worker" (Installed Global Name)
        let installed_worker = exe_path.join("transcribe-worker");
        
        // 2. Try "worker" (Development / Cargo Target Name)
        let dev_worker = exe_path.join("worker");
        
        // 3. Fallback for cargo run
        let fallback_worker = PathBuf::from("target/release/worker");

        let worker_cmd = if installed_worker.exists() {
            installed_worker.clone()
        } else if dev_worker.exists() {
            dev_worker.clone()
        } else {
            fallback_worker
        };

        if !worker_cmd.exists() {
             anyhow::bail!("Worker binary not found. Looked at {:?} and {:?}", installed_worker, dev_worker);
        }

        let child = Command::new(worker_cmd)
            .arg(model_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Let worker logs go to stderr
            .spawn()
            .context("Failed to spawn worker process")?;

        Ok(Self { child })
    }

    fn transcribe(&mut self, chunk_path: &Path) -> Result<Vec<TranscriptionSegment>> {
        let request = WorkerRequest {
            chunk_path: chunk_path.to_string_lossy().to_string(),
        };
        let json_req = serde_json::to_string(&request)?;

        let stdin = self.child.stdin.as_mut().context("Worker stdin not captured")?;
        writeln!(stdin, "{}", json_req)?;
        stdin.flush()?;

        let stdout = self.child.stdout.as_mut().context("Worker stdout not captured")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let response: WorkerResponse = serde_json::from_str(&line)
            .context("Failed to parse worker response")?;

        if response.success {
            Ok(response.segments.unwrap_or_default())
        } else {
            anyhow::bail!("Worker error: {}", response.error.unwrap_or_default())
        }
    }
}

/// Convert input audio to 16kHz, 16-bit, Mono WAV using FFmpeg
async fn convert_to_wav(input_path: &Path, output_path: &Path) -> Result<()> {
    println!("Converting {:?} to {:?}", input_path, output_path);
    
    let status = tokio::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(output_path)
        .arg("-y") // Overwrite output
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("Failed to execute ffmpeg")?;

    if !status.success() {
        anyhow::bail!("FFmpeg conversion failed");
    }

    Ok(())
}

#[derive(Clone)]
struct AudioChunk {
    path: PathBuf,
    start_time: f64,
    index: usize,
}

fn split_audio(wav_path: &Path, output_dir: &Path) -> Result<Vec<AudioChunk>> {
    println!("Splitting audio {:?}...", wav_path);
    
    let mut reader = hound::WavReader::open(wav_path).context("Failed to open WAV file")?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    
    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<_, _>>()?;
    let total_samples = samples.len();
    let samples_per_chunk = (sample_rate * CHUNK_DURATION) as usize;
    let samples_overlap = (sample_rate * OVERLAP_DURATION) as usize;
    let step_size = samples_per_chunk - samples_overlap;
    
    let mut chunks = Vec::new();

    let mut start_sample = 0;
    let mut index = 0;

    while start_sample < total_samples {
        let end_sample = std::cmp::min(start_sample + samples_per_chunk, total_samples);
        let chunk_samples = &samples[start_sample..end_sample];
        
        if chunk_samples.len() < sample_rate as usize && index > 0 {
            break;
        }

        let chunk_path = output_dir.join(format!("chunk_{}.wav", index));
        let start_time = start_sample as f64 / sample_rate as f64;
        
        let mut writer = hound::WavWriter::create(&chunk_path, spec)?;
        for sample in chunk_samples {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;
        
        chunks.push(AudioChunk {
            path: chunk_path,
            start_time,
            index,
        });

        start_sample += step_size;
        index += 1;
    }

    Ok(chunks)
}

fn format_timestamp(seconds: f32) -> String {
    let hours = (seconds / 3600.0) as u32;
    let minutes = ((seconds % 3600.0) / 60.0) as u32;
    let secs = (seconds % 60.0) as u32;
    let millis = ((seconds.fract()) * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
}

fn cleanup_text(text: &str) -> String {
    let mut text = text.to_string();
    
    // Common hallucination / typo fixes
    let replacements = vec![
        ("Start using a trial version of", ""),
        ("Unicorn", ""), // Often hallucinated in silence
        ("Amara.org", ""),
        ("Subtitle by", ""),
    ];

    for (target, replacement) in replacements {
        text = text.replace(target, replacement);
    }
    
    text.trim().to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let start_total = Instant::now();
    
    let args: Vec<String> = std::env::args().collect();
    let input_path = PathBuf::from(&args[1]);
    
    // Default model handling with fallback search
    let default_model_name = "models/breeze-asr-25-q4_k.bin";
    let model_arg = args.get(2).map(|s| s.as_str());
    
    // We check a list of potential bases for the model
    let mut potential_models = Vec::new();
    
    // 1. User provided path (if any)
    if let Some(s) = model_arg {
        if !s.starts_with("--") {
            potential_models.push(PathBuf::from(s));
        }
    }

    // 2. Current Directory
    potential_models.push(PathBuf::from(default_model_name));

    // 3. Absolute Codebase Path (For Global execution stability)
    // This is hardcoded for your specific setup to ensure it ALWAYS works on your Mac
    potential_models.push(PathBuf::from("/Users/leochen/Documents/chinese-transcibe").join(default_model_name));

    // 4. Executable Directory (if we ever move the models with the binary)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            potential_models.push(parent.join(default_model_name));
        }
    }

    let mut model_path = PathBuf::from(default_model_name); // Default for error msg
    let mut found = false;

    for path in potential_models {
        if path.exists() {
            model_path = path;
            found = true;
            break;
        }
    }
    
    if !found {
         eprintln!("Error: Model file not found. Searched in several locations. Last attempt: {:?}", model_path);
         eprintln!("Please ensure '{}' exists in your project folder.", default_model_name);
         std::process::exit(1);
    }

    // Output Setup
    let output_dir = PathBuf::from("outputs");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }
    
    // Determine filenames
    let file_stem = input_path.file_stem().context("Invalid filename")?;
    let output_srt = output_dir.join(file_stem).with_extension("srt");
    let output_txt = output_dir.join(file_stem).with_extension("txt");

    // Temp setup
    let temp_dir = TempDir::new()?;
    let converted_path = temp_dir.path().join("converted.wav");

    // 1. Convert
    convert_to_wav(&input_path, &converted_path).await?;

    // 2. Split
    let temp_path = temp_dir.path().to_path_buf();
    let chunks = task::spawn_blocking(move || {
        split_audio(&converted_path, &temp_path)
    }).await??;

    if chunks.is_empty() {
        println!("No audio chunks created.");
        return Ok(());
    }

    // 3. Start Workers
    println!("Starting {} persistent workers...", NUM_WORKERS);
    let (pool_tx, pool_rx) = async_channel::bounded(NUM_WORKERS);
    
    for i in 0..NUM_WORKERS {
        println!("Initializing Worker {}...", i + 1);
        let worker = WorkerProcess::new(&model_path)?;
        pool_tx.send_blocking(worker)?;
    }
    println!("All workers ready.");

    // 4. Distribute Work (Worker Pool Pattern)
    let results: Vec<Result<(AudioChunk, Vec<TranscriptionSegment>)>> = chunks.par_iter().map(|chunk| {
        let mut worker = pool_rx.recv_blocking().context("Failed to acquire worker from pool")?;
        print!("."); 
        std::io::stdout().flush().ok();
        let res = worker.transcribe(&chunk.path).map(|segments| (chunk.clone(), segments));
        pool_tx.send_blocking(worker).ok();
        res
    }).collect();

    println!("\nTranscription finished. Merging results...");

    let mut srt_file = fs::File::create(&output_srt)?;
    let mut global_index = 1;
    let mut all_text = String::new();
    
    for result in results {
       match result {
           Ok((chunk, segments)) => {
               for segment in segments {
                    if chunk.index > 0 && segment.start < OVERLAP_DURATION as f32 { continue; }

                    let abs_start = chunk.start_time as f32 + segment.start;
                    let abs_end = chunk.start_time as f32 + segment.end;
                    let text = cleanup_text(&segment.text);

                    if text.is_empty() { continue; }

                    writeln!(srt_file, "{}", global_index)?;
                    writeln!(srt_file, "{} --> {}", format_timestamp(abs_start), format_timestamp(abs_end))?;
                    writeln!(srt_file, "{}", text)?;
                    writeln!(srt_file)?;

                    if !all_text.is_empty() { all_text.push('\n'); }
                    all_text.push_str(&text);

                    global_index += 1;
               }
           },
           Err(e) => {
               eprintln!("\n⚠️  Error processing chunk: {}", e);
           }
       }
    }

    // 6. Output TXT if requested
    // 6. Output TXT (Always)
    fs::write(&output_txt, all_text)?;
    println!("Saved text to {:?}", output_txt);
    println!("Saved SRT to {:?}", output_srt);

    // 7. Archive Input File
    let completed_dir = PathBuf::from("completed");
    if !completed_dir.exists() {
        fs::create_dir_all(&completed_dir)?;
    }
    
    let dest_path = completed_dir.join(input_path.file_name().unwrap());
    println!("Moving source file to {:?}", dest_path);
    fs::rename(&input_path, &dest_path).context("Failed to move source file to completed folder")?;

    println!("Full Workflow Complete.");
    println!("Total Time: {:.2?}", start_total.elapsed());

    Ok(())
}
