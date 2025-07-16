#!/bin/bash

set -ex -o pipefail

cd "$(dirname "$0")/.."

if [ "$1" == "--help" ]; then
    echo "Usage: run.sh [test_name]"
    exit 0
fi

if [ "$BUILD_MODE" = "release" ]; then
    CARGO_ARGS="--release"
else
    BUILD_MODE="debug"
    CARGO_ARGS=""
fi

stage1() {
    mkdir -p target/.empty
    docker build --file tests/builder.Dockerfile --tag widgem_builder target/.empty
}

stage2() {
    docker run \
        --mount "type=bind,src=$PWD,dst=/app" \
        widgem_builder \
        "command -v rustup || \
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
                sh -s -- --default-toolchain 1.87.0 --profile minimal -y
            cargo build --package widgem_tests --locked $CARGO_ARGS"
    mkdir -p target/docker/bin
    cp target/docker/target/$BUILD_MODE/widgem_tests tests/xfce_entrypoint.sh target/docker/bin/
}

stage3() {
    docker build --file tests/tests.Dockerfile --tag widgem_tests target/docker/bin
}

stage4() {
    docker rm --force widgem_tests || true
    docker run --name widgem_tests \
        --mount "type=bind,source=$PWD,target=/app" \
        --publish 25901:5901 --publish 26901:6901 \
        widgem_tests \
        widgem_tests test "$1"
}

if [ "$STAGE" = "2" ]; then
    stage2
elif [ "$STAGE" = "4" ]; then
    stage4
else
    stage1
    stage2
    stage3
    stage4
fi
