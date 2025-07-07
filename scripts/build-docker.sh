#!/bin/bash

set -e

# Build script for CSD Docker image
# Usage: ./scripts/build-docker.sh [tag]

TAG=${1:-"conversadocs/csd:latest"}
BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

echo "🔨 Building CSD Docker image..."
echo "📦 Tag: $TAG"
echo "📅 Build Date: $BUILD_DATE"
echo "🏷️  Version: $VERSION"

# Build the Docker image
docker build \
    --tag "$TAG" \
    --tag "conversadocs/csd:$VERSION" \
    --label "org.opencontainers.image.created=$BUILD_DATE" \
    --label "org.opencontainers.image.version=$VERSION" \
    --label "org.opencontainers.image.title=Code-Scan-Docs" \
    --label "org.opencontainers.image.description=A comprehensive code analysis and documentation tool" \
    --label "org.opencontainers.image.source=https://github.com/conversadocs/csd" \
    --label "org.opencontainers.image.licenses=MIT" \
    .

echo "✅ Build completed successfully!"
echo "🚀 Run: docker run --rm -v \$(pwd):/app/code $TAG init"
echo "📋 Or: docker run --rm $TAG --help"
