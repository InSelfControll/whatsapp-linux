# Build stage for WhatsApp Desktop wrapper
# Use latest stable Rust
FROM rust:latest AS builder

# Install all required dependencies for dioxus-desktop on Linux
RUN apt-get update && apt-get install -y \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libxdo-dev \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libgtk-3-dev \
    libsoup-3.0-dev \
    libjavascriptcoregtk-4.1-dev \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the project files
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY assets ./assets

# Build release binary
RUN cargo build --release

# The binary will be at /app/target/release/whatsapp-desktop
