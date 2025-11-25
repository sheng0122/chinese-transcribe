# transcribe-rs

A Rust library for audio transcription using the Whisper engine.

This library was extracted from the [Handy](https://github.com/cjpais/handy) project to help other developers integrate transcription capabilities into their applications.

## Features

- **Whisper Engine**: Support for OpenAI's Whisper model
- **Cross-platform**: Works on macOS, Windows, and Linux with optimized backends
- **Hardware Acceleration**: Metal on macOS, Vulkan on Windows/Linux
- **Flexible API**: Common interface for transcription
- **Server Implementation**: Includes a ready-to-use REST API server

## Required Model Files

**Whisper Model:**
- Single GGML file (e.g., `breeze-asr-25-q4_k`)

**Audio Requirements:**
- Format: WAV
- Sample Rate: 16 kHz
- Channels: Mono (1 channel)
- Bit Depth: 16-bit
- Encoding: PCM

## Model Downloads

- **Whisper**: https://huggingface.co/ggerganov/whisper.cpp/tree/main

## Usage

```rust
use transcribe_rs::{TranscriptionEngine, engines::whisper::WhisperEngine};
use std::path::PathBuf;

let mut engine = WhisperEngine::new();
engine.load_model(&PathBuf::from("path/to/model.bin"))?;
let result = engine.transcribe_file(&PathBuf::from("audio.wav"), None)?;
println!("{}", result.text);
```

## Running the Example

### Setup

1. **Create the models directory:**
   ```bash
   mkdir models
   ```

2. **Download Whisper Model:**
   ```bash
   # Download Whisper model
   cd models
   wget https://blob.handy.computer/whisper-medium-q4_1.bin
   cd ..
   ```

### Running the Example

```bash
cargo run --example transcribe
```

The example will:
- Load the Whisper model
- Transcribe `samples/dots.wav`
- Display timing information and transcription results

## Acknowledgments

- Thanks to the [whisper.cpp](https://github.com/ggerganov/whisper.cpp) project for the Whisper implementation
