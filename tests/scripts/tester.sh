#!/bin/bash

# Run widgem tester for widgem's own tests

set -ex -o pipefail

cd "$(dirname "$0")/../.."

if [[ "$OSTYPE" =~ ^darwin ]]; then
    cargo bundle --package widgem_tester --bin widgem_tester
    rm -f ./target/stderr ./target/stdout
    open "./target/debug/bundle/osx/Widgem tester.app" \
        --stderr ./target/stderr \
        --stdout ./target/stdout \
        --args \
        "$PWD/tests" --run-script "$PWD/tests/scripts/run_in_docker.sh"
    tail -f ./target/stderr ./target/stdout
else
    cargo run --bin widgem_tester -- tests --run-script tests/scripts/run_in_docker.sh
fi
