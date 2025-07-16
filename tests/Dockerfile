# syntax=docker/dockerfile:1
# check=skip=SecretsUsedInArgOrEnv

ARG BUILD_MODE=release
FROM docker.io/lukemathwalker/cargo-chef:0.1.71-rust-1.87.0-bookworm AS chef
WORKDIR /app
COPY rust-toolchain ./
RUN rustc --version
ENV CARGO_TERM_COLOR=always

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY widgem/Cargo.toml widgem/Cargo.toml
COPY macros/Cargo.toml macros/Cargo.toml
COPY tests/Cargo.toml tests/Cargo.toml
COPY uitest/Cargo.toml uitest/Cargo.toml
COPY test_kit/Cargo.toml test_kit/Cargo.toml
RUN mkdir macros/src uitest/src test_kit/src widgem/src tests/src && \
    touch macros/src/lib.rs uitest/src/lib.rs test_kit/src/lib.rs widgem/src/lib.rs tests/src/main.rs
RUN cargo chef prepare

FROM chef AS builder
ARG BUILD_MODE
COPY --from=planner /app/recipe.json .
RUN if [ $BUILD_MODE = "release" ]; then \
        echo "--release" > cargo_args; \
    elif [ $BUILD_MODE = "debug" ]; then \
        echo > cargo_args; \
    else \
        echo "Unknown build mode"; \
        exit 1; \
    fi; \
    echo cargo args: $(cat cargo_args)
RUN apt-get update && apt-get install -y \
    build-essential pkg-config libclang-dev libxcb1-dev libxrandr-dev \
    libdbus-1-dev libpipewire-0.3-dev libwayland-dev libegl-dev \
    libgbm-dev
RUN cargo chef cook $(cat cargo_args)
COPY Cargo.toml Cargo.lock ./
COPY widgem widgem
COPY macros macros
COPY uitest uitest
COPY test_kit test_kit
COPY tests/src tests/src
COPY tests/Cargo.toml tests/
RUN cargo build --package widgem_tests --locked $(cat cargo_args)

FROM docker.io/ubuntu:noble-20250415.1 AS xfce
ENV USER=root
ENV DISPLAY=:1
ENV VNC_PASSWORD=1
ENV LC_ALL=en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US.UTF-8
RUN apt-get update && apt-get install -y \
    tigervnc-standalone-server xfce4 xfce4-terminal dbus-x11 \
    xdotool wmctrl libxkbcommon-x11-0 locales locales-all && \
    mkdir /root/.vnc && \
    printf '#!/bin/bash\nstartxfce4\n' > /root/.vnc/xstartup && \
    chmod +x /root/.vnc/xstartup && \
    echo "$VNC_PASSWORD" | vncpasswd -f > /root/.vnc/passwd && \
    chmod 0600 /root/.vnc/passwd && \
    touch /root/.Xauthority
COPY tests/xfce_entrypoint.sh /entrypoint
ENTRYPOINT ["/entrypoint"]

FROM xfce AS runtime
ARG BUILD_MODE
ENV RUST_BACKTRACE=1
ENV WIDGEM_REPO_DIR=/widgem
COPY --from=builder /app/target/$BUILD_MODE/widgem_tests /usr/local/bin/
