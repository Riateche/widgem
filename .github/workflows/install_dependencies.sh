#!/bin/bash

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [[ $RUNNER_OS == "Linux" ]]; then
    sudo apt-get update && sudo apt-get install -y \
        libxcb1-dev libxrandr-dev \
        libdbus-1-dev libpipewire-0.3-dev libwayland-dev libegl-dev \
        libgbm-dev
fi
