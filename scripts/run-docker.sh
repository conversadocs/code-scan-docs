#!/bin/bash

set -e

# Run script for CSD Docker container
# Usage: ./scripts/run-docker.sh [command] [args...]
# Examples:
#   ./scripts/run-docker.sh init
#   ./scripts/run-docker.sh docs --format markdown
#   ./scripts/run-docker.sh plugins --detailed

# Configuration
IMAGE_NAME="conversadocs/csd:latest"
CODE_DIR="${CSD_CODE_DIR:-$(pwd)}"
OUTPUT_DIR="${CSD_OUTPUT_DIR:-$(pwd)/csd-output}"
CACHE_DIR="${CSD_CACHE_DIR:-$(pwd)/.csd_cache}"

# Create output and cache directories if they don't exist
mkdir -p "$OUTPUT_DIR" "$CACHE_DIR"

echo "🚀 Running CSD Docker container..."
echo "📁 Code directory: $CODE_DIR"
echo "📤 Output directory: $OUTPUT_DIR"
echo "💾 Cache directory: $CACHE_DIR"

# Run the container with volume mounts
docker run --rm \
    -v "$CODE_DIR:/app/code:ro" \
    -v "$OUTPUT_DIR:/app/.csd_output" \
    -v "$CACHE_DIR:/app/.csd_cache" \
    -e RUST_LOG="${RUST_LOG:-info}" \
    "$IMAGE_NAME" "$@"
