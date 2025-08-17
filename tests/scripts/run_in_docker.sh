#!/bin/bash

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [ "$BUILD_MODE" = "release" ]; then
    CARGO_ARGS="--release"
else
    BUILD_MODE="debug"
    CARGO_ARGS=""
fi

docker run \
    --mount "type=bind,src=$PWD,dst=/app" \
    widgem_builder \
    "command -v rustup || \
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- --default-toolchain 1.87.0 --profile minimal -y
        cargo build --package widgem_tests --locked $CARGO_ARGS"

docker exec \
    widgem_tests \
    /app/target/docker/target/$BUILD_MODE/widgem_tests $*
