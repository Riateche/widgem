FROM docker.io/lukemathwalker/cargo-chef:0.1.67-rust-slim-bullseye AS chef
COPY rust-toolchain ./
RUN rustc --version
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY salvation/Cargo.toml salvation/Cargo.toml
COPY macros/Cargo.toml macros/Cargo.toml
COPY tests/Cargo.toml tests/Cargo.toml
COPY uitest/Cargo.toml uitest/Cargo.toml
RUN mkdir macros/src uitest/src salvation/src tests/src && \
    touch macros/src/lib.rs uitest/src/lib.rs salvation/src/lib.rs tests/src/main.rs
RUN cargo chef prepare

FROM chef AS builder
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release
RUN apt-get update && apt-get install -y libxcb-xfixes0-dev
COPY Cargo.toml Cargo.lock ./
COPY salvation salvation
COPY macros macros
COPY uitest uitest
COPY uitest uitest
COPY tests/src tests/src
COPY tests/Cargo.toml tests/
RUN cargo build --package salvation_tests --locked --release

FROM docker.io/neuroidss/ubuntu-xfce-vnc:20.04 AS runtime
USER root
RUN apt-get update && apt-get install -y xdotool wmctrl
COPY --from=builder /app/target/release/salvation_tests /usr/local/bin/
ENV RUST_BACKTRACE=1
ENV SALVATION_REPO_DIR=/salvation
