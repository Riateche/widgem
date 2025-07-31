#!/bin/bash

set -ex -o pipefail

cd "$(dirname "$0")/../.."

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

mkdir -p target/.empty
docker build --file tests/scripts/builder.Dockerfile --tag widgem_builder target/.empty

docker run \
    --mount "type=bind,src=$PWD,dst=/app" \
    widgem_builder \
    "command -v rustup || \
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
            sh -s -- --default-toolchain 1.87.0 --profile minimal -y
        cargo build --package widgem_tests --locked $CARGO_ARGS"
mkdir -p target/docker/bin
# cp target/docker/target/$BUILD_MODE/widgem_tests tests/scripts/xfce_entrypoint.sh target/docker/bin/

docker build --file tests/scripts/tests.Dockerfile --tag widgem_tests tests/scripts

docker rm --force widgem_tests || true
docker run --name widgem_tests \
    --mount "type=bind,source=$PWD/target/docker/target/$BUILD_MODE/widgem_tests,target=/usr/local/bin/widgem_tests" \
    --mount "type=bind,source=$PWD,target=/app" \
    --publish 25901:5901 \
    -it \
    widgem_tests \
    widgem_tests test "$1"
