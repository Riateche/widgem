#!/bin/bash

# Build and run tests in a docker container.
# Arguments are passed to the test runner.

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [ "$BUILD_MODE" = "release" ]; then
    CARGO_ARGS="--release"
else
    BUILD_MODE="debug"
    CARGO_ARGS=""
fi

# Check if widgem_builder image exists.
if [ -z "$(docker images -q widgem_builder 2> /dev/null)" ]; then
    ./tests/scripts/setup_docker.sh
fi
# Check if widgem_xfce container is running.
if [ -z "$(docker ps -q -f name=widgem_xfce)" ]; then
    ./tests/scripts/setup_docker.sh
fi

# Build test binary.
docker run \
    --mount "type=bind,src=$PWD,dst=/app" \
    widgem_builder \
    "command -v rustup || \
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- --default-toolchain 1.87.0 --profile minimal -y
        cargo build --package widgem_tests --locked $CARGO_ARGS"

# Run test binary in the widgem_xfce container.
docker exec \
    widgem_xfce \
    /app/target/docker/target/$BUILD_MODE/widgem_tests $*
