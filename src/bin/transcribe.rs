use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use transcribe_rs::worker::transcribe_parallel;

/// Convert input audio to 16kHz, 16-bit, Mono WAV using FFmpeg
fn convert_to_wav(input_path: &Path, output_path: &Path) -> Result<()> {
    println!("Converting {:?} to {:?}", input_path, output_path);
    
    let status = std::process::Command::new("ffmpeg")
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
        .status()
        .context("Failed to execute ffmpeg")?;

    if !status.success() {
        anyhow::bail!("FFmpeg conversion failed");
    }

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_audio> <model_path>", args[0]);
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let model_path = PathBuf::from(&args[2]);
    
    // Output SRT file name
    let output_srt_path = input_path.with_extension("srt");

    // Temp WAV file name
    let temp_wav_path = input_path.with_extension("temp_16k.wav");

    // 1. Convert to WAV
    convert_to_wav(&input_path, &temp_wav_path)?;

    // 2. Transcribe
    println!("Starting transcription...");
    let srt_content = transcribe_parallel(&temp_wav_path, &model_path)?;

    // 3. Write SRT
    std::fs::write(&output_srt_path, srt_content)?;
    println!("Transcription saved to {:?}", output_srt_path);

    // Cleanup
    std::fs::remove_file(temp_wav_path).ok();

    Ok(())
}
