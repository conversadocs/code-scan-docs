# ========== BUILD STAGE ==========
FROM rust:1.82.0 AS builder

RUN apt-get update && apt-get install -y \
    python3 python3-pip python3-venv \
    libssl-dev pkg-config git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /csd
COPY . .

# Build the Rust core
RUN cargo build --release

# ========== RUNTIME STAGE ==========
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    python3 python3-pip ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy Rust binary
COPY --from=builder /csd/target/release/csd /usr/local/bin/csd

# Copy Python plugins and dependencies
COPY --from=builder /csd/plugins/ /opt/csd/plugins/

WORKDIR /opt/csd

# Install Python plugin requirements (if any)
# RUN if [ -f requirements.txt ]; then pip3 install --no-cache-dir -r requirements.txt; fi

# Skip pip install because there’s no requirements file
RUN echo "No Python dependencies to install"

ENTRYPOINT ["csd"]
CMD ["--help"]
