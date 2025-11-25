use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use async_channel;
use crate::engines::whisper::{WhisperEngine, WhisperInferenceParams};
use crate::{TranscriptionEngine, TranscriptionSegment};
use crate::subtitle::generate_srt;
use crate::audio::get_audio_duration;
use rayon::prelude::*;
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Queued,
    Processing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: String,
    pub status: TaskStatus,
    pub original_filename: String,
    #[serde(skip)]
    pub file_path: PathBuf,
    #[serde(skip)]
    pub result_srt: Option<String>,
}

pub type TaskMap = Arc<Mutex<HashMap<String, Task>>>;

pub struct Worker {
    worker_id: usize,
    task_receiver: async_channel::Receiver<String>,
    tasks: TaskMap,
    model_path: PathBuf,
}

impl Worker {
    pub fn new(
        worker_id: usize,
        task_receiver: async_channel::Receiver<String>,
        tasks: TaskMap,
        model_path: PathBuf,
    ) -> Self {
        Self {
            worker_id,
            task_receiver,
            tasks,
            model_path,
        }
    }

    pub async fn run(self) {
        println!("[Worker {}] Started. Waiting for tasks...", self.worker_id);
        
        // Load model once (in a real scenario, we might want to load/unload or have a pool)
        // For this MVP, we'll load it for each task or keep it loaded. 
        // Since WhisperEngine is not Send, we have to be careful.
        // Simplest approach for MVP: Load model inside the loop for each task (slow) or keep it in a separate thread.
        // Better: Spawn a blocking thread for the heavy lifting.

        while let Ok(task_id) = self.task_receiver.recv().await {
            let tasks = self.tasks.clone();
            let model_path = self.model_path.clone();
            let worker_id = self.worker_id;

            // Update status to Processing
            {
                let mut map = tasks.lock().unwrap();
                if let Some(task) = map.get_mut(&task_id) {
                    task.status = TaskStatus::Processing;
                    println!("[Worker {}] Processing task: {}", worker_id, task_id);
                }
            }

            // Execute transcription in a blocking thread
            let task_id_clone = task_id.clone();
            let result = tokio::task::spawn_blocking(move || -> Result<String> {
                let task_data = {
                    let map = tasks.lock().unwrap();
                    map.get(&task_id_clone).cloned().context("Task not found")?
                };

                // 1. Convert to WAV (reuse logic or call ffmpeg directly)
                let temp_wav = task_data.file_path.with_extension("wav");
                convert_to_wav(&task_data.file_path, &temp_wav)?;

                // 2. Parallel Transcription
                let srt = transcribe_parallel(&temp_wav, &model_path)?;

                // Cleanup
                std::fs::remove_file(temp_wav).ok();

                Ok(srt)
            }).await;

            // Update status based on result
            let mut map = self.tasks.lock().unwrap();
            if let Some(task) = map.get_mut(&task_id) {
                match result {
                    Ok(Ok(srt_content)) => {
                        task.status = TaskStatus::Completed;
                        task.result_srt = Some(srt_content);
                        println!("[Worker {}] Task {} completed successfully.", self.worker_id, task_id);
                    }
                    Ok(Err(e)) => {
                        task.status = TaskStatus::Failed(e.to_string());
                        eprintln!("[Worker {}] Task {} failed: {}", self.worker_id, task_id, e);
                    }
                    Err(e) => {
                        task.status = TaskStatus::Failed(format!("Worker panic: {}", e));
                        eprintln!("[Worker {}] Task {} panicked: {}", self.worker_id, task_id, e);
                    }
                }
            }
        }
    }
}

// Helper: FFmpeg conversion (Synchronous for blocking thread)
fn convert_to_wav(input: &PathBuf, output: &PathBuf) -> Result<()> {
    let status = std::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(input)
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(output)
        .arg("-y")
        .status()
        .context("Failed to run ffmpeg")?;

    if !status.success() {
        anyhow::bail!("FFmpeg conversion failed");
    }
    Ok(())
}

struct Chunk {
    index: usize,
    start_time: f64,
    end_time: f64,
    temp_file: PathBuf,
}

pub fn transcribe_parallel(wav_path: &PathBuf, model_path: &PathBuf) -> Result<String> {
    let duration = get_audio_duration(wav_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Configuration
    let chunk_duration = 300.0; // 5 minutes
    let overlap = 10.0; // 10 seconds overlap
    let num_threads = 3;

    let mut chunks = Vec::new();
    let mut current_time = 0.0;
    let mut index = 0;

    // Create chunks plan
    while current_time < duration {
        let start = current_time;
        let end = (start + chunk_duration).min(duration);
        
        let temp_file = wav_path.with_file_name(format!("{}_chunk_{}.wav", wav_path.file_stem().unwrap().to_string_lossy(), index));

        chunks.push(Chunk {
            index,
            start_time: start,
            end_time: end,
            temp_file,
        });

        current_time += chunk_duration;
        index += 1;
    }

    println!("Splitting audio into {} chunks for parallel processing...", chunks.len());

    // Use std::sync::mpsc for job distribution and result collection
    let (job_tx, job_rx) = std::sync::mpsc::channel();
    let job_rx = Arc::new(Mutex::new(job_rx)); // Share receiver among threads
    let (result_tx, result_rx) = std::sync::mpsc::channel();

    // Spawn workers
    let mut handles = Vec::new();
    for i in 0..num_threads {
        let job_rx = job_rx.clone();
        let result_tx = result_tx.clone();
        let model_path = model_path.clone();
        let wav_path = wav_path.clone();
        
        handles.push(std::thread::spawn(move || {
            println!("[Thread {}] Loading model...", i);
            let mut engine = WhisperEngine::new();
            if let Err(e) = engine.load_model(&model_path) {
                eprintln!("[Thread {}] Failed to load model: {}", i, e);
                return;
            }
            println!("[Thread {}] Model loaded. Ready for tasks.", i);

            loop {
                // Fetch next job
                let chunk: Chunk = {
                    let rx = job_rx.lock().unwrap();
                    match rx.recv() {
                        Ok(job) => job,
                        Err(_) => break, // Channel closed, no more jobs
                    }
                };

                // Process chunk
                // 1. Extract chunk using FFmpeg
                let phys_start = (chunk.start_time - overlap).max(0.0);
                let phys_end = (chunk.end_time + overlap).min(duration);
                let phys_duration = phys_end - phys_start;

                let status = std::process::Command::new("ffmpeg")
                    .arg("-i")
                    .arg(&wav_path)
                    .arg("-ss")
                    .arg(format!("{:.3}", phys_start))
                    .arg("-t")
                    .arg(format!("{:.3}", phys_duration))
                    .arg("-c")
                    .arg("copy") // Fast copy since it's already WAV
                    .arg(&chunk.temp_file)
                    .arg("-y")
                    .output();

                match status {
                    Ok(output) if output.status.success() => {
                        // 2. Transcribe
                        let params = WhisperInferenceParams::default();
                        match engine.transcribe_file(&chunk.temp_file, Some(params)) {
                            Ok(transcription) => {
                                // 3. Cleanup temp file
                                std::fs::remove_file(&chunk.temp_file).ok();

                                // 4. Process segments (Filter and Offset)
                                let mut valid_segments = Vec::new();
                                if let Some(segments) = transcription.segments {
                                    for mut seg in segments {
                                        // Adjust timestamp to global time (seconds)
                                        seg.start += phys_start as f32;
                                        seg.end += phys_start as f32;

                                        // Filter based on core range (seconds)
                                        let core_start = chunk.start_time as f32;
                                        let core_end = chunk.end_time as f32;

                                        // Simple inclusion check: if the segment's midpoint is within the core range
                                        let midpoint = (seg.start + seg.end) / 2.0;
                                        if midpoint >= core_start && midpoint < core_end {
                                            valid_segments.push(seg);
                                        }
                                    }
                                }
                                result_tx.send(Ok(valid_segments)).ok();
                            }
                            Err(e) => {
                                result_tx.send(Err(anyhow::anyhow!("Transcription failed: {}", e))).ok();
                            }
                        }
                    }
                    _ => {
                        result_tx.send(Err(anyhow::anyhow!("FFmpeg extraction failed"))).ok();
                    }
                }
            }
        }));
    }

    // Send jobs
    let total_chunks = chunks.len();
    for chunk in chunks {
        job_tx.send(chunk).unwrap();
    }
    drop(job_tx); // Close channel so workers know when to stop

    // Collect results
    let mut all_segments = Vec::new();
    for _ in 0..total_chunks {
        match result_rx.recv() {
            Ok(Ok(segments)) => all_segments.extend(segments),
            Ok(Err(e)) => eprintln!("Chunk processing error: {}", e),
            Err(e) => eprintln!("Failed to receive result: {}", e),
        }
    }

    // Wait for threads to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Sort by start time
    all_segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(Ordering::Equal));

    // Generate SRT
    Ok(generate_srt(&all_segments))
}
