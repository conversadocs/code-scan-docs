# Multi-stage build for Code-Scan-Docs (CSD)
# Stage 1: Rust build environment
FROM rust:1.75-slim AS rust-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy Rust project files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the Rust binary in release mode
RUN cargo build --release

# Stage 2: Python environment setup
FROM python:3.11-slim AS python-builder

# Install system dependencies for Python packages
RUN apt-get update && apt-get install -y \
  build-essential \
  && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy Python plugin SDK and requirements
COPY plugins/shared/python/ ./plugins/shared/python/
COPY plugins/shared/python/requirements.txt ./

# Install Python dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Install the plugin SDK
RUN pip install --no-cache-dir ./plugins/shared/python/

# Stage 3: Final runtime image
FROM python:3.11-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  ca-certificates \
  git \
  && rm -rf /var/lib/apt/lists/* \
  && apt-get clean

# Create non-root user for security
RUN groupadd -r csd && useradd -r -g csd -d /app -s /bin/bash csd

# Set working directory
WORKDIR /app

# Copy the compiled Rust binary from builder stage
COPY --from=rust-builder /build/target/release/csd /usr/local/bin/csd

# Copy Python environment from python-builder stage
COPY --from=python-builder /usr/local/lib/python3.11/site-packages/ /usr/local/lib/python3.11/site-packages/
COPY --from=python-builder /usr/local/bin/ /usr/local/bin/

# Copy plugin files
COPY plugins/ ./plugins/
COPY .csdrc.yaml ./

# Create necessary directories
RUN mkdir -p .csd_cache .csd_output

# Set proper permissions
RUN chown -R csd:csd /app /usr/local/bin/csd

# Switch to non-root user
USER csd

# Set environment variables
ENV PYTHONPATH="/app/plugins/shared/python:$PYTHONPATH"
ENV RUST_LOG=info
ENV CSD_CACHE_DIR="/app/.csd_cache"
ENV CSD_OUTPUT_DIR="/app/.csd_output"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD csd --version || exit 1

# Set the default working directory for mounted code
WORKDIR /app/code

# Default command
ENTRYPOINT ["csd"]
CMD ["--help"]
