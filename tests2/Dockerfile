FROM docker.io/lukemathwalker/cargo-chef:0.1.67-rust-slim-bullseye as chef
WORKDIR /app

FROM chef AS planner
COPY . ./
RUN cargo chef prepare

FROM chef AS builder
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release
RUN apt-get update && apt-get install -y libxcb-xfixes0-dev
COPY . ./
RUN cargo build --package salvation_tests2 --release

FROM docker.io/neuroidss/ubuntu-xfce-vnc:20.04 AS runtime
USER root
RUN apt-get update && apt-get install -y xdotool wmctrl
COPY --from=builder /app/target/release/salvation_tests2 /usr/local/bin/
ENV RUST_BACKTRACE=1
ENV SALVATION_REPO_DIR=/salvation
