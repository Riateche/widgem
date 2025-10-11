#!/bin/bash

# Run widgem tester for widgem's own tests.
# Run tests on the host system.

set -ex -o pipefail

cd "$(dirname "$0")/../.."
cargo run --bin widgem_tester -- tests
