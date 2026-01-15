#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SERVER_BINARY="$PROJECT_ROOT/target/release/license-server"
if [ ! -f "$SERVER_BINARY" ]; then
    echo "Binaire introuvable. Lancez ./build-all.sh"
    exit 1
fi

if [ "$EUID" -ne 0 ]; then
    echo "Ce script doit être exécuté en tant que root."
    exit 1
fi

mkdir -p /var/lib/license-server /var/log/license-server
chmod 755 /var/lib/license-server /var/log/license-server

cp "$SERVER_BINARY" /usr/bin/license-server
chmod 755 /usr/bin/license-server
chown root:root /usr/bin/license-server

echo "OK: /usr/bin/license-server installé. Utilisez /usr/bin/license-server --help"
