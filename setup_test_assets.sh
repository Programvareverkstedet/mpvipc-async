#!/usr/bin/env bash

set -euo pipefail

REQUIRED_COMMANDS=(
  "git"
  "ffmpeg"
)

for cmd in "${REQUIRED_COMMANDS[@]}"; do
  if ! command -v "$cmd" &> /dev/null; then
    echo "Command '$cmd' not found. Please install it and try again."
    exit 1
  fi
done

ROOT_DIR=$(git rev-parse --show-toplevel)

# Generate 30 seconds of 480p video with black background

ffmpeg -f lavfi -i color=c=black:s=640x480:d=30 -c:v libx264 -t 30 -pix_fmt yuv420p "$ROOT_DIR/test_assets/black-background-30s-480p.mp4"