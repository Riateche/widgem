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

# Check if widgem_xfce container is running.
if [ -z "$(docker ps -q -f name=widgem_xfce)" ]; then
    ./tests/scripts/setup_docker.sh
fi

if [[ -z "$CI" ]]; then
    # Check if widgem_builder image exists.
    if [ -z "$(docker images -q widgem_builder 2> /dev/null)" ]; then
        ./tests/scripts/setup_docker.sh
    fi

    # Build test binary in docker.
    docker run \
        --mount "type=bind,src=$PWD,dst=/app" \
        widgem_builder \
        "command -v rustup || \
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
                sh -s -- --default-toolchain 1.87.0 --profile minimal -y
            cargo build --package widgem_tests --locked $CARGO_ARGS"

    BIN_DIR="/app/target/docker/target/$BUILD_MODE"
else
    # Build test binary.
    cargo build --package widgem_tests --locked

    BIN_DIR="/app/target/$BUILD_MODE"
fi

# Run test binary in the widgem_xfce container.
if [[ $# -gt 0 ]]; then
    docker exec widgem_xfce "$BIN_DIR/widgem_tests" $*
else
    docker exec widgem_xfce "$BIN_DIR/widgem_tests" test
    RESULT=$(docker exec widgem_xfce "$BIN_DIR/work_area")
    EXPECTED="[(0, 27, 1600, 873)]"
    if [ "$RESULT" = "$EXPECTED" ]; then
        echo "Correct"
    else
        echo "Expected '$EXPECTED', got '$RESULT'"
        exit 1
    fi
fi

