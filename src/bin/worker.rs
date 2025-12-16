use anyhow::{Context, Result};
use std::path::PathBuf;
use std::io::{self, BufRead, Write};
use transcribe_rs::engines::whisper::{WhisperEngine, WhisperInferenceParams};
use transcribe_rs::{TranscriptionEngine, TranscriptionSegment};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct WorkerRequest {
    chunk_path: String,
}

#[derive(Serialize, Deserialize)]
struct WorkerResponse {
    success: bool,
    segments: Option<Vec<TranscriptionSegment>>,
    error: Option<String>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <model_path>", args[0]);
        std::process::exit(1);
    }

    let model_path = PathBuf::from(&args[1]);

    // 1. Load model ONCE
    eprintln!("[Worker] Loading model from {:?}...", model_path);
    let mut engine = WhisperEngine::new();
    engine.load_model(&model_path)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to load model")?;
    eprintln!("[Worker] Model loaded. Ready for input.");

    // 2. Event Loop
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match handle.read_line(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = buffer.trim();
                if line.is_empty() { continue; }
                
                // Parse Request
                let request: WorkerRequest = match serde_json::from_str(line) {
                    Ok(req) => req,
                    Err(e) => {
                        send_error(format!("JSON Parse Error: {}", e));
                        continue;
                    }
                };

                // Transcribe
                eprintln!("[Worker] Transcribing {:?}", request.chunk_path);
                let params = WhisperInferenceParams::default();
                let path = PathBuf::from(request.chunk_path);
                
                match engine.transcribe_file(&path, Some(params)) {
                    Ok(result) => {
                        let response = WorkerResponse {
                            success: true,
                            segments: result.segments, // Can be None
                            error: None,
                        };
                        send_response(&response);
                    },
                    Err(e) => {
                        send_error(format!("Transcription failed: {}", e));
                    }
                }
            }
            Err(e) => {
                eprintln!("[Worker] Error reading stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn send_response(response: &WorkerResponse) {
    if let Ok(json) = serde_json::to_string(response) {
        println!("{}", json);
        io::stdout().flush().unwrap();
    }
}

fn send_error(msg: String) {
    let response = WorkerResponse {
        success: false,
        segments: None,
        error: Some(msg),
    };
    send_response(&response);
}
