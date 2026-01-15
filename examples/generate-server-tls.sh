#!/bin/bash

set -euo pipefail

OUT_DIR="${1:-/etc/license-server}"
CERT_PATH="${OUT_DIR}/server.crt"
KEY_PATH="${OUT_DIR}/server.key"
DAYS="${DAYS:-3650}"
SUBJ="${SUBJ:-/CN=license-server}"

if ! command -v openssl &> /dev/null; then
    echo "openssl est requis."
    exit 1
fi

mkdir -p "$OUT_DIR"

openssl req -x509 -newkey rsa:2048 \
    -keyout "$KEY_PATH" \
    -out "$CERT_PATH" \
    -days "$DAYS" \
    -nodes \
    -subj "$SUBJ"

chmod 600 "$KEY_PATH"
chmod 644 "$CERT_PATH"

echo "OK: $CERT_PATH et $KEY_PATH"
