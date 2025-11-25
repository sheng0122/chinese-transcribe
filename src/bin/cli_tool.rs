use anyhow::{Context, Result};
use hound;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::task;
use transcribe_rs::engines::whisper::{WhisperEngine, WhisperInferenceParams};
use transcribe_rs::TranscriptionEngine;

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
        .status()
        .await
        .context("Failed to execute ffmpeg")?;

    if !status.success() {
        anyhow::bail!("FFmpeg conversion failed");
    }

    Ok(())
}

/// Split audio into chunks of `duration_secs`
fn split_audio(wav_path: &Path, duration_secs: u32, output_dir: &Path) -> Result<Vec<(PathBuf, f64)>> {
    println!("Splitting audio {:?} into {}s chunks", wav_path, duration_secs);
    
    let mut reader = hound::WavReader::open(wav_path).context("Failed to open WAV file")?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let samples_per_chunk = (sample_rate * duration_secs) as usize;
    
    if spec.channels != 1 || spec.sample_rate != 16000 {
        anyhow::bail!("Input WAV must be 16kHz Mono");
    }

    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<_, _>>()?;
    let _total_samples = samples.len();
    let mut chunks = Vec::new();

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    for (i, chunk_samples) in samples.chunks(samples_per_chunk).enumerate() {
        let chunk_path = output_dir.join(format!("chunk_{}.wav", i));
        let start_time = (i * samples_per_chunk) as f64 / sample_rate as f64;
        
        let mut writer = hound::WavWriter::create(&chunk_path, spec)?;
        for sample in chunk_samples {
            writer.write_sample(*sample)?;
        }
        writer.finalize()?;
        
        chunks.push((chunk_path, start_time));
    }

    println!("Created {} chunks", chunks.len());
    Ok(chunks)
}

/// Core transcription function (Async wrapper)
async fn transcribe_chunk(
    engine: &mut WhisperEngine, 
    chunk_path: &Path, 
    chunk_start_time: f64
) -> Result<String> {
    println!("Transcribing chunk: {:?}", chunk_path);
    
    // Since WhisperEngine is synchronous, we run it directly here.
    
    let params = WhisperInferenceParams::default();
    let result = engine.transcribe_file(chunk_path, Some(params))
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Transcription failed")?;

    let mut formatted_output = String::new();
    if let Some(segments) = result.segments {
        for segment in segments {
            // Adjust timestamp relative to the full audio
            let abs_start = chunk_start_time + segment.start as f64;
            let abs_end = chunk_start_time + segment.end as f64;
            formatted_output.push_str(&format!("[{:.2}s - {:.2}s]: {}\n", abs_start, abs_end, segment.text));
        }
    } else {
        formatted_output.push_str(&format!("[{:.2}s]: {}\n", chunk_start_time, result.text));
    }

    Ok(formatted_output)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_audio> <model_path>", args[0]);
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let model_path = PathBuf::from(&args[2]);
    let temp_dir = PathBuf::from("temp_chunks");
    let converted_path = temp_dir.join("converted.wav");

    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir)?;
    }

    // 1. Convert to WAV
    convert_to_wav(&input_path, &converted_path).await?;

    // 2. Split Audio
    // We run this blocking task in a blocking thread to avoid blocking the async runtime
    let chunks = task::spawn_blocking(move || {
        split_audio(&converted_path, 60, &temp_dir)
    }).await??;

    // 3. Initialize Engine
    println!("Loading model from {:?}", model_path);
    let mut engine = WhisperEngine::new();
    engine.load_model(&model_path)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to load model")?;

    // 4. Transcribe Chunks
    println!("Starting transcription...");
    let mut full_transcript = String::new();

    for (chunk_path, start_time) in chunks {
        let chunk_text = transcribe_chunk(&mut engine, &chunk_path, start_time).await?;
        print!("{}", chunk_text); // Stream output to stdout
        full_transcript.push_str(&chunk_text);
    }

    // Cleanup
    // fs::remove_dir_all(temp_dir)?;

    println!("\nFull Transcription Complete.");
    Ok(())
}
