#!/bin/bash
set -e

echo "🚀 Building Transcribe-RS for production..."
cargo build --release

echo "📦 Installing binaries to /usr/local/bin..."

# Ensure destination exists
# sudo mkdir -p /usr/local/bin

# Copy binaries
sudo cp target/release/cli_tool /usr/local/bin/transcribe
sudo cp target/release/worker /usr/local/bin/transcribe-worker

echo "✅ Installed!"
echo "   Command: transcribe"
echo "   Worker : transcribe-worker (internal use)"
echo ""
echo "📝 Usage: transcribe <audio_file>"
