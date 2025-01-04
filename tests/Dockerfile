ARG BUILD_MODE=release
FROM docker.io/lukemathwalker/cargo-chef:0.1.67-rust-slim-bullseye AS chef
WORKDIR /app
COPY rust-toolchain ./
RUN rustc --version

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY salvation/Cargo.toml salvation/Cargo.toml
COPY macros/Cargo.toml macros/Cargo.toml
COPY tests/Cargo.toml tests/Cargo.toml
COPY uitest/Cargo.toml uitest/Cargo.toml
COPY test_kit/Cargo.toml test_kit/Cargo.toml
RUN mkdir macros/src uitest/src test_kit/src salvation/src tests/src && \
    touch macros/src/lib.rs uitest/src/lib.rs test_kit/src/lib.rs salvation/src/lib.rs tests/src/main.rs
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
RUN cargo chef cook $(cat cargo_args)
RUN apt-get update && apt-get install -y libxcb-xfixes0-dev
COPY Cargo.toml Cargo.lock ./
COPY salvation salvation
COPY macros macros
COPY uitest uitest
COPY test_kit test_kit
COPY tests/src tests/src
COPY tests/Cargo.toml tests/
RUN cargo build --package salvation_tests --locked $(cat cargo_args)

FROM docker.io/neuroidss/ubuntu-xfce-vnc:20.04 AS runtime
ARG BUILD_MODE
USER root
RUN apt-get update && apt-get install -y xdotool wmctrl
COPY --from=builder /app/target/$BUILD_MODE/salvation_tests /usr/local/bin/
ENV RUST_BACKTRACE=1
ENV SALVATION_REPO_DIR=/salvation
