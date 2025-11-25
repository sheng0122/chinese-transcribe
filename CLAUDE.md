# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

transcribe-rs is a Rust library providing unified transcription capabilities supporting multiple speech recognition engines:
- **Whisper**: Local inference using whisper-rs (GGML models)
- **Parakeet**: NVIDIA NeMo Parakeet using ONNX Runtime
- **OpenAI API**: Remote transcription via OpenAI's API

## Build Commands

```bash
cargo build                      # Build library
cargo build --release            # Release build
cargo run --example transcribe   # Run local engines example
cargo run --example openai       # Run OpenAI API example
cargo run --bin cli_tool <audio> <model_path>  # CLI for long audio
cargo run --bin server           # Start HTTP API server
```

## Testing

```bash
cargo test                       # Run all tests
cargo test --test whisper        # Run whisper tests only
cargo test --test parakeet       # Run parakeet tests only
cargo test --test openai         # Run OpenAI tests (requires OPENAI_API_KEY)
cargo test test_jfk_transcription # Run single test by name
RUST_LOG=debug cargo test -- --nocapture  # Run with logging
```

Tests require models in `models/` directory (not committed to repo):
- Whisper: `models/whisper-medium-q4_1.bin`
- Parakeet: `models/parakeet-tdt-0.6b-v3-int8/`

Model downloads:
- Parakeet: https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/tree/main
- Whisper: https://huggingface.co/ggerganov/whisper.cpp/tree/main

## Code Quality

```bash
cargo fmt      # Format code
cargo clippy   # Lint
cargo check    # Type check without building
cargo test --doc  # Test documentation examples
```

## Architecture

### Core Trait
All engines implement `TranscriptionEngine` trait (`src/lib.rs`):
- `load_model()` / `load_model_with_params()` - Load model from path
- `transcribe_samples()` - Transcribe f32 audio samples
- `transcribe_file()` - Transcribe WAV file directly
- Associated types: `InferenceParams` and `ModelParams` for engine-specific configuration

Remote engines use `RemoteTranscriptionEngine` (async variant).

### Module Structure
- `src/audio.rs` - WAV file reading and validation
- `src/engines/whisper.rs` - Whisper engine using whisper-rs
- `src/engines/parakeet/` - Parakeet engine (ONNX-based)
  - `engine.rs` - Main implementation
  - `model.rs` - ONNX model wrapper
  - `timestamps.rs` - Timestamp processing
- `src/remote/openai.rs` - OpenAI API client

### Audio Requirements
All engines expect: 16kHz, 16-bit, mono PCM WAV files. Samples are normalized to f32 in [-1.0, 1.0].

### Audio Processing Note
The library processes audio as a whole file. For Parakeet, the time resolution is 80ms per step (10ms window × 8 subsampling factor). Segmentation into sentences happens post-inference based on punctuation.

### Binaries for Long Audio

**cli_tool** (`src/bin/cli_tool.rs`):
- Converts any audio to 16kHz mono WAV via FFmpeg
- Splits into **60-second chunks** (no overlap)
- Transcribes sequentially, adjusting timestamps per chunk
- Usage: `cargo run --bin cli_tool <audio_file> <model_path>`

**server** (`src/bin/server.rs`):
- Actix-web HTTP API on port 8080
- Endpoints: `POST /upload`, `GET /status/{id}`, `GET /download/{id}`
- Background worker processes tasks via mpsc channel
- Test with `./test_api.sh <audio_file>`

### Platform-Specific Backends
Whisper uses Metal on macOS, Vulkan on Windows/Linux (configured in Cargo.toml features).

### Test Patterns
- Whisper tests share a single loaded model via `Lazy<Mutex<WhisperEngine>>` to avoid reloading
- OpenAI tests are async (`#[tokio::test]`) and require `OPENAI_API_KEY` environment variable
