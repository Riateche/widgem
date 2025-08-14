#!/bin/bash

set -ex -o pipefail

cd "$(dirname "$0")/../.."

mkdir -p target/.empty
docker build --file tests/scripts/builder.Dockerfile --tag widgem_builder target/.empty

docker build --file tests/scripts/tests.Dockerfile --tag widgem_tests tests/scripts

docker rm --force widgem_tests || true
docker run --detach \
    --name widgem_tests \
    --mount "type=bind,source=$PWD,target=/app" \
    --publish 25901:5901 \
    widgem_tests \
    sleep infinity
