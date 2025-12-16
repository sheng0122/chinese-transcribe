# transcribe-rs

A high-performance, parallel audio transcription tool written in Rust, powered by OpenAI's Whisper engine.

## Features

- **Parallel Transcription**: Uses a multi-process architecture to transcribe long audio files significantly faster.
- **Process Isolation**: Ensures stability by isolating Metal (GPU) contexts in separate worker processes.
- **Smart Chunking**: Automatically splits audio into 5-minute chunks with overlap to ensure context preservation.
- **Cross-platform**: Optimized for Apple Silicon (Metal) on macOS.

## Prerequisites

1.  **Rust Toolchain**: Ensure you have Rust installed.
2.  **FFmpeg**: Required for audio conversion.
    ```bash
    brew install ffmpeg
    ```

## Installation & Build

Clone the repository and build the project in release mode for maximum performance:

```bash
# Build both the CLI tool and the worker binary
cargo build --release --bin cli_tool
cargo build --release --bin worker
```

## Usage

To transcribe an audio file, you can use the provided helper script `transcribe.sh`. It defaults to using the `models/breeze-asr-25-q4_k.bin` model.

```bash
# Syntax
./transcribe.sh <audio_file_path> [model_path]

# Example (using default model)
./transcribe.sh "uploads/podcast_ep1.mp3"

# Example (specifying a different model)
./transcribe.sh "uploads/podcast_ep1.mp3" "models/other_model.bin"
```

### Batch Transcription

To transcribe all audio files in a directory (supports mp3, wav, m4a), use `transcribe_folder.sh`:

```bash
# Syntax
./transcribe_folder.sh <directory_path> [model_path]

# Example
./transcribe_folder.sh "mp3"
```

Alternatively, you can run the cargo command directly:
```bash
cargo run --release --bin cli_tool "uploads/podcast_ep1.mp3" "models/breeze-asr-25-q4_k.bin"
```

### Output
The tool will generate a SubRip Subtitle (`.srt`) file in the same directory as the input audio file.

## Architecture

This tool uses a **Coordinator-Worker** architecture:
- **CLI Tool (Coordinator)**: Splits audio, manages the thread pool, and aggregates results.
- **Worker**: Independent processes that handle the actual transcription of single chunks. This design bypasses stability issues associated with multi-threaded Metal usage.

## Required Model Files

- **Whisper Model**: Single GGML format file (e.g., `breeze-asr-25-q4_k.bin`).
- Download models from [Hugging Face](https://huggingface.co/ggerganov/whisper.cpp/tree/main).

## Acknowledgments

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) for the core Whisper implementation.
