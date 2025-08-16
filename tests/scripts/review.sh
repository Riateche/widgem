#!/bin/bash

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [[ "$OSTYPE" =~ ^darwin ]]; then
    cargo bundle --package widgem_tester --bin widgem_tester --release
    rm -f ./target/stderr ./target/stdout
    open "./target/release/bundle/osx/Widgem tester.app" \
        --stderr ./target/stderr \
        --stdout ./target/stdout \
        --args \
        "$PWD/tests" --run-script "$PWD/tests/scripts/run_in_docker.sh"
else
    cargo run --bin widgem_tester --release -- tests --run-script tests/scripts/run_in_docker.sh
fi
