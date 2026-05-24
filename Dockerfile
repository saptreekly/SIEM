# Dockerfile for SIEM Ensemble Development
FROM ubuntu:24.04

# Avoid prompts
ENV DEBIAN_FRONTEND=noninteractive

# Core system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    wget \
    cmake \
    llvm-18 \
    llvm-18-dev \
    clang-18 \
    libluajit-5.1-dev \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Zig (v0.16.0)
RUN curl -LO https://ziglang.org/download/0.16.0/zig-x86_64-linux-0.16.0.tar.xz && \
    tar -xf zig-x86_64-linux-0.16.0.tar.xz && \
    mv zig-x86_64-linux-0.16.0 /opt/zig && \
    ln -s /opt/zig/zig /usr/local/bin/zig && \
    rm zig-x86_64-linux-0.16.0.tar.xz

# Build Odin from Source
RUN git clone https://github.com/odin-lang/Odin /opt/Odin && \
    cd /opt/Odin && \
    LLVM_CONFIG=llvm-config-18 make release-native && \
    ln -s /opt/Odin/odin /usr/local/bin/odin

# Install Elixir/Erlang
RUN apt-get update && apt-get install -y elixir && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace/SIEM

# Expose ports for SIEM and control plane
EXPOSE 9090 9000

CMD ["/bin/bash"]
