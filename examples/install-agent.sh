#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [ "$EUID" -ne 0 ]; then
    echo "Ce script doit être exécuté en tant que root."
    exit 1
fi

AGENT_BINARY="$PROJECT_ROOT/target/release/license-agent"
CLI_BINARY="$PROJECT_ROOT/target/release/license-agent-cli"

if [ ! -f "$AGENT_BINARY" ] || [ ! -f "$CLI_BINARY" ]; then
    echo "Binaires introuvables. Lancez ./build-all.sh"
    exit 1
fi

if ! id "license-agent" &>/dev/null; then
    useradd -r -s /bin/false -d /var/lib/license-agent license-agent
fi

mkdir -p /etc/license-agent /etc/licence-agent
mkdir -p /var/lib/license-agent /var/log/license-agent /var/run/license-agent

chown -R license-agent:license-agent /var/lib/license-agent /var/log/license-agent /var/run/license-agent
chmod 755 /etc/license-agent /etc/licence-agent /var/lib/license-agent /var/log/license-agent /var/run/license-agent

cp "$AGENT_BINARY" /usr/bin/license-agent
cp "$CLI_BINARY" /usr/bin/license-agent-cli
chmod 755 /usr/bin/license-agent /usr/bin/license-agent-cli
chown root:root /usr/bin/license-agent /usr/bin/license-agent-cli

if [ ! -f /etc/license-agent/config.toml ] && [ -f "$PROJECT_ROOT/deploy/config.toml.example" ]; then
    cp "$PROJECT_ROOT/deploy/config.toml.example" /etc/license-agent/config.toml
    chmod 640 /etc/license-agent/config.toml
    chown root:license-agent /etc/license-agent/config.toml
fi

if [ -f "$PROJECT_ROOT/deploy/license-agent.service" ]; then
    cp "$PROJECT_ROOT/deploy/license-agent.service" /etc/systemd/system/license-agent.service
    systemctl daemon-reload
fi

echo "OK: agent installé. Configurez /etc/license-agent/config.toml puis démarrez le service."
