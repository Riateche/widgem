#!/bin/bash

# Build widgem_builder and widgem_xfce images.
# Start widgem_xfce container.

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [[ -z $CI ]]; then
    # widgem_builder image is used to build test binaries for widgem_xfce environment while
    # keeping cache and build artifacts in target/docker, not relying on the docker's cache.
    mkdir -p target/.empty
    docker build --file tests/scripts/builder.Dockerfile --tag widgem_builder target/.empty
fi

# widgem_xfce is an Ubuntu image with XFCE environment and a VNC server.
# It's used to run tests in a reproducible way.
# It's possible to connect to the VNC server at vnc://127.0.0.1:25901
# with password "1".
docker build --file tests/scripts/xfce.Dockerfile --tag widgem_xfce tests/scripts

docker rm --force widgem_xfce || true
# Run widgem_xfce in background.
docker run --detach \
    --name widgem_xfce \
    --mount "type=bind,source=$PWD,target=/app" \
    --publish 25901:5901 \
    widgem_xfce \
    sleep infinity
