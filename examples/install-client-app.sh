#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CLIENT_BINARY="$PROJECT_ROOT/target/release/client-app"
if [ ! -f "$CLIENT_BINARY" ]; then
    echo "Binaire introuvable. Lancez ./build-all.sh"
    exit 1
fi

if [ "$EUID" -ne 0 ]; then
    echo "Ce script doit être exécuté en tant que root."
    exit 1
fi

cp "$CLIENT_BINARY" /usr/bin/client-app
chmod 755 /usr/bin/client-app
chown root:root /usr/bin/client-app

echo "OK: /usr/bin/client-app installé. Utilisez /usr/bin/client-app --help"
