#!/bin/bash

set -ex

cd "$(dirname "$0")/.."

if [ "$1" == "--help" ]; then
    echo "Usage: run.sh [test_name]"
    exit 0
fi

if [[ -z "${GITHUB_ACTIONS}" ]]; then
    docker build \
        --tag widgem_tests \
        --file tests/Dockerfile \
        --build-arg BUILD_MODE \
        --progress plain \
        .
else
    echo "Skipping docker build in Github Actions"
fi

docker rm --force widgem_tests || true
docker run --name widgem_tests \
    --mount "type=bind,source=$PWD,target=/widgem" \
    --publish 25901:5901 --publish 26901:6901 \
    widgem_tests \
    widgem_tests test "$1"

# for i in {1..20}; do
#     sleep 0.3
#     echo Testing container status
#     docker exec widgem_tests xdotool click 1 || true
#     if docker exec widgem_tests xdotool getactivewindow; then
#        echo Container is ready
#        break
#     fi
#     if ! docker exec widgem_tests pidof xfwm4; then
#         echo xfwm4 is not running, starting xfwm4
#         docker exec --detach widgem_tests xfwm4
#     fi
# done
# if [ "$i" == "20" ]; then
#     2>&1 echo "Container check failed"
#     exit 1
# fi
# docker exec widgem_tests widgem_tests test "$1"
