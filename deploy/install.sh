#!/bin/bash
set -euo pipefail

# Script d'installation License Secret Agent

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Installation License Secret Agent ==="

# Vérifier root
if [ "$EUID" -ne 0 ]; then 
    echo "Erreur: Ce script doit être exécuté en tant que root"
    exit 1
fi

# 1. Créer utilisateur
echo "Création utilisateur license-agent..."
if ! id "license-agent" &>/dev/null; then
    useradd -r -s /bin/false -d /var/lib/license-agent license-agent
    echo "Utilisateur license-agent créé"
else
    echo "Utilisateur license-agent existe déjà"
fi

# 2. Créer répertoires
echo "Création répertoires..."
mkdir -p /etc/license-agent
mkdir -p /var/lib/license-agent
mkdir -p /var/log/license-agent
mkdir -p /usr/bin

# 3. Permissions
echo "Configuration permissions..."
chown -R license-agent:license-agent /var/lib/license-agent
chown -R license-agent:license-agent /var/log/license-agent
chmod 755 /etc/license-agent
chmod 755 /var/lib/license-agent
chmod 755 /var/log/license-agent

# 4. Copier binaire
echo "Installation binaire..."
if [ -f "$PROJECT_ROOT/target/release/license-agent" ]; then
    cp "$PROJECT_ROOT/target/release/license-agent" /usr/bin/license-agent
    chmod 755 /usr/bin/license-agent
    chown root:root /usr/bin/license-agent
    echo "Binaire installé"
else
    echo "ATTENTION: Binaire non trouvé. Compilez d'abord avec: cargo build --release"
    exit 1
fi

# 5. Copier configuration exemple
if [ ! -f /etc/license-agent/config.toml ]; then
    echo "Installation configuration exemple..."
    cp "$SCRIPT_DIR/config.toml.example" /etc/license-agent/config.toml
    chmod 600 /etc/license-agent/config.toml
    chown root:root /etc/license-agent/config.toml
    echo "Configuration exemple installée. MODIFIEZ /etc/license-agent/config.toml avant de démarrer"
else
    echo "Configuration existe déjà, non modifiée"
fi

# 6. Installer service systemd
echo "Installation service systemd..."
cp "$SCRIPT_DIR/license-agent.service" /etc/systemd/system/license-agent.service
systemctl daemon-reload
echo "Service systemd installé"

# 7. Résumé
echo ""
echo "=== Installation terminée ==="
echo ""
echo "Prochaines étapes:"
echo "1. Modifier /etc/license-agent/config.toml avec vos paramètres"
echo "2. Installer certificats client dans /etc/license-agent/"
echo "3. Démarrer le service: systemctl start license-agent"
echo "4. Activer au démarrage: systemctl enable license-agent"
echo "5. Vérifier statut: systemctl status license-agent"
echo ""
