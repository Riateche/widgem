#!/bin/bash

# Run widgem tester for widgem's own tests.
# Run tests in docker.

set -ex -o pipefail

cd "$(dirname "$0")/../.."
cargo run --bin widgem_tester -- tests --run-script tests/scripts/run_tests_in_docker.sh
