#!/bin/bash

# Check if directory is provided
if [ -z "$1" ]; then
    echo "Usage: ./transcribe_folder.sh <directory_path> [model_path]"
    echo "Example: ./transcribe_folder.sh mp3"
    exit 1
fi

TARGET_DIR="$1"
MODEL_PATH="$2"

# Check if directory exists
if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: Directory '$TARGET_DIR' does not exist."
    exit 1
fi

echo "Scanning directory: $TARGET_DIR"

# Find audio files (mp3, wav, m4a) and process them one by one
# Using -print0 and IFS= read -r -d '' to correctly handle filenames with spaces and special characters
find "$TARGET_DIR" -type f \( -name "*.mp3" -o -name "*.wav" -o -name "*.m4a" \) -print0 | while IFS= read -r -d '' file; do
    echo "----------------------------------------------------------------"
    echo "Transcribing: $file"
    ./transcribe.sh "$file" "$MODEL_PATH"
done

echo "----------------------------------------------------------------"
echo "Batch transcription complete."
