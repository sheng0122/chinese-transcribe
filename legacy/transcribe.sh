#!/bin/bash

# Default model path
MODEL_PATH="models/breeze-asr-25-q4_k.bin"

# Check if audio file is provided
if [ -z "$1" ]; then
    echo "Usage: ./transcribe.sh <audio_file_path> [model_path]"
    echo "Example: ./transcribe.sh uploads/podcast.mp3"
    exit 1
fi

# Allow overriding model path
if [ ! -z "$2" ]; then
    MODEL_PATH="$2"
fi

# Check if model exists
if [ ! -f "$MODEL_PATH" ]; then
    echo "Error: Model file not found at $MODEL_PATH"
    echo "Please download the model or specify a valid path."
    exit 1
fi

# Run the tool
# We use --quiet to reduce cargo output, but keep the tool's output
cargo run --release --quiet --bin cli_tool "$1" "$MODEL_PATH"
