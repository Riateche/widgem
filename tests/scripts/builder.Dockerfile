FROM docker.io/ubuntu:noble-20250415.1
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libclang-dev libxcb1-dev libxrandr-dev \
    libdbus-1-dev libpipewire-0.3-dev libwayland-dev libegl-dev \
    libgbm-dev curl
ENV CARGO_TERM_COLOR=always
ENV CARGO_TARGET_DIR=/app/target/docker/target
ENV CARGO_HOME=/app/target/docker/cargo
ENV RUSTUP_HOME=/app/target/docker/rustup
ENV PATH="$PATH:$CARGO_HOME/bin"
RUN mkdir /app
WORKDIR /app
ENTRYPOINT ["/bin/bash", "-o", "pipefail", "-exc"]
