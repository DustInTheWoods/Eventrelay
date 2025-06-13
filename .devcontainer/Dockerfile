# Use the official Rust image
FROM rust:latest

# Install additional development tools
RUN apt-get update && apt-get install -y \
    git \
    curl \
    wget \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set up workspace directory
WORKDIR /workspace

# Install additional Rust components
RUN rustup component add rustfmt clippy

# Pre-create Cargo directory to avoid permission issues
RUN mkdir -p /usr/local/cargo/registry

# Set environment variables
ENV RUST_BACKTRACE=1

# Default command
CMD ["bash"]