#!/bin/bash

# Build and run tests in a docker container.
# Arguments are passed to the test runner.
#
# Variables:
# BUILD_MODE ("debug" or "release", default = "debug")

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [[ $BUILD_MODE == "release" ]]; then
    CARGO_ARGS="--release"
else
    BUILD_MODE="debug"
    CARGO_ARGS=""
fi

# Check if widgem_xfce container is running.
if [[ -z $(docker ps -q -f name=widgem_xfce) ]]; then
    ./tests/scripts/setup_docker.sh
fi

if [[ -z $CI ]]; then
    # Host OS may not be compatible with Docker environment,
    # so we build test binary using widgem_builder Docker image.

    # Check if widgem_builder image exists.
    if [[ -z $(docker images -q widgem_builder 2> /dev/null) ]]; then
        ./tests/scripts/setup_docker.sh
    fi

    # Build test binary in docker.
    docker run \
        --mount "type=bind,src=$PWD,dst=/app" \
        --env "NO_COLOR=$NO_COLOR" \
        --env "CARGO_TERM_COLOR=$CARGO_TERM_COLOR" \
        widgem_builder \
        "command -v rustup || \
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- --default-toolchain 1.87.0 --profile minimal -y
        cargo build --package widgem_tests --locked $CARGO_ARGS"

    BIN_DIR="/app/target/docker/target/$BUILD_MODE"
else
    # CI uses the same OS as widgem_xfce image, so we can build test binary
    # directly on the current system.
    cargo build --package widgem_tests --locked $CARGO_ARGS

    BIN_DIR="/app/target/$BUILD_MODE"
fi

# Run test binaries in the widgem_xfce container.
DOCKER_TEST_CMD=$(echo \
    docker exec \
        --env "NO_COLOR=$NO_COLOR" \
        --env "CARGO_TERM_COLOR=$CARGO_TERM_COLOR" \
        widgem_xfce "$BIN_DIR/widgem_tests"
)
if [[ $# -gt 0 ]]; then
    $DOCKER_TEST_CMD $*
else
    $DOCKER_TEST_CMD test

    # Run extra tests (Docker only)

    RESULT="$(docker exec widgem_xfce "$BIN_DIR/work_area")"
    EXPECTED="[(0, 27, 1600, 873)]"
    { set +x; } 2>/dev/null
    if [ "$RESULT" != "$EXPECTED" ]; then
        echo "Expected '$EXPECTED', got '$RESULT'"
        exit 1
    fi
    set -x

    test_work_area() {
        CONFIG="$1"
        EXPECTED="$2"
        RESULT="$(
            docker run --rm \
                --name widgem_xfce2 \
                --mount "type=bind,source=$PWD,target=/app" \
                --mount "type=bind,source=$PWD/tests/xfce/xfce4-panel-$CONFIG.xml,target=/root/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml" \
                --publish 25902:5901 \
                widgem_xfce \
                "$BIN_DIR/work_area" \
            | tail -1
        )"
        { set +x; } 2>/dev/null
        if [ "$RESULT" != "$EXPECTED" ]; then
            echo "Expected '$EXPECTED', got '$RESULT'"
            exit 1
        fi
        set -x
    }

    test_work_area top50 "[(0, 51, 1600, 849)]"
    # 900 - 51 - 26 = 823
    test_work_area top50-bottom25 "[(0, 51, 1600, 823)]"

    test_work_area left26-bottom48 "[(27, 0, 1573, 851)]"
    test_work_area right26-bottom48 "[(0, 0, 1573, 851)]"
    test_work_area middle-vertical-bottom48 "[(0, 0, 1600, 851)]"

    { set +x; } 2>/dev/null
    echo "extra tests succeeded"
    set -x
fi
