#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"
cargo build --release --no-default-features

cd "$SCRIPT_DIR/license-server"
cargo build --release

cd "$SCRIPT_DIR/client-app"
cargo build --release

echo "OK: binaires dans $PROJECT_ROOT/target/release"
