#!/bin/bash
set -euo pipefail

# Script de construction d'un tarball pour déploiement
# Crée une archive tar.gz avec les binaires et fichiers nécessaires

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/tarball"
VERSION="${1:-0.1.0}"
ARCH="${2:-$(uname -m)}"

echo "=== Construction tarball License Secret Agent ==="
echo "Version: $VERSION"
echo "Architecture: $ARCH"
echo ""

# Nettoyer build précédent
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# 1. Compiler le projet
echo "1. Compilation du projet..."
cd "$PROJECT_ROOT"
cargo build --release

if [ ! -f "target/release/license-agent" ] || [ ! -f "target/release/license-agent-cli" ]; then
    echo "ERREUR: Binaires non trouvés après compilation"
    exit 1
fi

echo "✓ Compilation réussie"

# 2. Créer structure du tarball
echo "2. Création structure tarball..."
TARBALL_DIR="$BUILD_DIR/license-secret-agent-${VERSION}-${ARCH}"
mkdir -p "$TARBALL_DIR/bin"
mkdir -p "$TARBALL_DIR/etc"
mkdir -p "$TARBALL_DIR/systemd"
mkdir -p "$TARBALL_DIR/scripts"

# 3. Copier binaires
echo "3. Copie des binaires..."
cp target/release/license-agent "$TARBALL_DIR/bin/"
cp target/release/license-agent-cli "$TARBALL_DIR/bin/"
chmod 755 "$TARBALL_DIR/bin/"*

# 4. Copier fichiers de configuration
echo "4. Copie fichiers configuration..."
cp deploy/config.toml.example "$TARBALL_DIR/etc/config.toml.example"
cp deploy/license-agent.service "$TARBALL_DIR/systemd/"

# 5. Copier scripts
echo "5. Copie scripts..."
cp deploy/install.sh "$TARBALL_DIR/scripts/"

# 6. Créer README pour le tarball
cat > "$TARBALL_DIR/README.txt" <<EOF
License Secret Agent ${VERSION} - ${ARCH}

INSTALLATION:
=============

1. Extraire l'archive:
   tar -xzf license-secret-agent-${VERSION}-${ARCH}.tar.gz
   cd license-secret-agent-${VERSION}-${ARCH}

2. Exécuter le script d'installation:
   sudo ./scripts/install.sh

OU installation manuelle:

1. Copier les binaires:
   sudo cp bin/* /usr/bin/

2. Créer utilisateur:
   sudo useradd -r -s /bin/false -d /var/lib/license-agent license-agent

3. Créer répertoires:
   sudo mkdir -p /etc/license-agent
   sudo mkdir -p /var/lib/license-agent
   sudo mkdir -p /var/log/license-agent

4. Copier configuration:
   sudo cp etc/config.toml.example /etc/license-agent/config.toml
   sudo chmod 600 /etc/license-agent/config.toml

5. Installer service systemd:
   sudo cp systemd/license-agent.service /etc/systemd/system/
   sudo systemctl daemon-reload

6. Configurer et démarrer:
   sudo nano /etc/license-agent/config.toml
   sudo systemctl start license-agent
   sudo systemctl enable license-agent

DEPENDANCES:
============
- libssl3 (ou openssl)
- systemd
- tpm2-tss (optionnel, recommandé)

VERSION: ${VERSION}
ARCHITECTURE: ${ARCH}
EOF

# 7. Créer le tarball
echo "6. Création archive..."
cd "$BUILD_DIR"
tar -czf "license-secret-agent-${VERSION}-${ARCH}.tar.gz" "license-secret-agent-${VERSION}-${ARCH}"

TARBALL_FILE="license-secret-agent-${VERSION}-${ARCH}.tar.gz"
TARBALL_SIZE=$(du -h "$TARBALL_FILE" | cut -f1)

echo ""
echo "✓ Tarball créé avec succès!"
echo "  Fichier: $BUILD_DIR/$TARBALL_FILE"
echo "  Taille: $TARBALL_SIZE"
echo ""
echo "Pour extraire:"
echo "  tar -xzf $BUILD_DIR/$TARBALL_FILE"
echo ""
