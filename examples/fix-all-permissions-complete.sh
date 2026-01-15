#!/bin/bash

set -euo pipefail

if [ "$EUID" -ne 0 ]; then
    echo "Ce script doit être exécuté en tant que root."
    exit 1
fi

if ! id "license-agent" &>/dev/null; then
    echo "Utilisateur license-agent manquant."
    exit 1
fi

CONFIG_PATH="/etc/license-agent/config.toml"
if [ ! -f "$CONFIG_PATH" ] && [ -f "/etc/licence-agent/config.toml" ]; then
    CONFIG_PATH="/etc/licence-agent/config.toml"
fi

if [ ! -f "$CONFIG_PATH" ]; then
    echo "config.toml introuvable."
    exit 1
fi

CONFIG_DIR="$(dirname "$CONFIG_PATH")"

chmod 640 "$CONFIG_PATH"
chown root:license-agent "$CONFIG_PATH"
chmod 755 "$CONFIG_DIR"

CLIENT_KEY=$(grep -E '^client_key' "$CONFIG_PATH" | cut -d'"' -f2 | head -1)
CLIENT_CERT=$(grep -E '^client_cert' "$CONFIG_PATH" | cut -d'"' -f2 | head -1)

if [ -n "$CLIENT_KEY" ] && [ -f "$CLIENT_KEY" ]; then
    chmod 640 "$CLIENT_KEY"
    chown root:license-agent "$CLIENT_KEY"
fi

if [ -n "$CLIENT_CERT" ] && [ -f "$CLIENT_CERT" ]; then
    chmod 644 "$CLIENT_CERT"
    chown root:license-agent "$CLIENT_CERT"
fi

mkdir -p /var/lib/license-agent /var/log/license-agent /var/run/license-agent
chown -R license-agent:license-agent /var/lib/license-agent /var/log/license-agent /var/run/license-agent
chmod 755 /var/lib/license-agent /var/log/license-agent /var/run/license-agent

IPC_SOCKET=$(grep -E '^ipc_socket_path' "$CONFIG_PATH" | cut -d'"' -f2 | head -1)
if [ -n "$IPC_SOCKET" ]; then
    IPC_DIR="$(dirname "$IPC_SOCKET")"
    mkdir -p "$IPC_DIR"
    chown license-agent:license-agent "$IPC_DIR"
    chmod 755 "$IPC_DIR"
    rm -f "$IPC_SOCKET"
fi

FALLBACK_STORAGE=$(grep -E '^fallback_encrypted_storage' "$CONFIG_PATH" | cut -d'"' -f2 | head -1)
if [ -n "$FALLBACK_STORAGE" ]; then
    FALLBACK_DIR="$(dirname "$FALLBACK_STORAGE")"
    if [ "$FALLBACK_DIR" != "." ]; then
        mkdir -p "$FALLBACK_DIR"
        chown license-agent:license-agent "$FALLBACK_DIR"
        chmod 755 "$FALLBACK_DIR"
    fi
    if [ -f "$FALLBACK_STORAGE" ]; then
        chown license-agent:license-agent "$FALLBACK_STORAGE"
        chmod 600 "$FALLBACK_STORAGE"
    fi
fi

echo "OK: permissions corrigées."
