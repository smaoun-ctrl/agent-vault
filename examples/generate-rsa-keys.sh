#!/bin/bash

set -euo pipefail

KEY_DIR="${1:-/etc/licence-agent}"
CLIENT_KEY="${KEY_DIR}/client.key"
CLIENT_CERT="${KEY_DIR}/client.pem"
SUBJ="${SUBJ:-/CN=license-agent-client}"

if ! command -v openssl &> /dev/null; then
    echo "openssl est requis."
    exit 1
fi

mkdir -p "$KEY_DIR"

openssl genrsa -out "$CLIENT_KEY" 2048

# Certificat client auto-signé (X.509) pour mTLS
openssl req -new -x509 -key "$CLIENT_KEY" -out "$CLIENT_CERT" -days 3650 -subj "$SUBJ"

chmod 600 "$CLIENT_KEY"
chmod 644 "$CLIENT_CERT"

echo "OK: $CLIENT_KEY et $CLIENT_CERT"
