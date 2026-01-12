#!/bin/bash
set -euo pipefail

# Script de construction de package d'installation
# Crée un package .deb pour Debian/Ubuntu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/package"
VERSION="${1:-0.1.0}"
ARCH="${2:-amd64}"

echo "=== Construction package License Secret Agent ==="
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

# 2. Créer structure du package
echo "2. Création structure package..."
PACKAGE_DIR="$BUILD_DIR/license-secret-agent_${VERSION}_${ARCH}"
mkdir -p "$PACKAGE_DIR/DEBIAN"
mkdir -p "$PACKAGE_DIR/usr/bin"
mkdir -p "$PACKAGE_DIR/usr/lib/systemd/system"
mkdir -p "$PACKAGE_DIR/etc/license-agent"
mkdir -p "$PACKAGE_DIR/usr/share/license-agent"
mkdir -p "$PACKAGE_DIR/var/lib/license-agent"
mkdir -p "$PACKAGE_DIR/var/log/license-agent"

# 3. Copier binaires
echo "3. Copie des binaires..."
cp target/release/license-agent "$PACKAGE_DIR/usr/bin/"
cp target/release/license-agent-cli "$PACKAGE_DIR/usr/bin/"
chmod 755 "$PACKAGE_DIR/usr/bin/license-agent"
chmod 755 "$PACKAGE_DIR/usr/bin/license-agent-cli"

# 4. Copier fichiers système
echo "4. Copie fichiers système..."
cp deploy/license-agent.service "$PACKAGE_DIR/usr/lib/systemd/system/"
cp deploy/config.toml.example "$PACKAGE_DIR/usr/share/license-agent/"
cp deploy/config.toml.example "$PACKAGE_DIR/etc/license-agent/config.toml.example"

# 5. Créer fichiers DEBIAN
echo "5. Création fichiers DEBIAN..."

# Control
sed "s/Version: .*/Version: ${VERSION}/" "$PROJECT_ROOT/deb/control" | \
    sed "s/Architecture: .*/Architecture: ${ARCH}/" > "$PACKAGE_DIR/DEBIAN/control"

# Scripts
cp "$PROJECT_ROOT/deb/postinst" "$PACKAGE_DIR/DEBIAN/"
cp "$PROJECT_ROOT/deb/prerm" "$PACKAGE_DIR/DEBIAN/"
cp "$PROJECT_ROOT/deb/postrm" "$PACKAGE_DIR/DEBIAN/"
chmod 755 "$PACKAGE_DIR/DEBIAN/postinst"
chmod 755 "$PACKAGE_DIR/DEBIAN/prerm"
chmod 755 "$PACKAGE_DIR/DEBIAN/postrm"

# 6. Créer le package .deb
echo "6. Construction package .deb..."
cd "$BUILD_DIR"
dpkg-deb --build "license-secret-agent_${VERSION}_${ARCH}"

if [ -f "license-secret-agent_${VERSION}_${ARCH}.deb" ]; then
    DEB_FILE="license-secret-agent_${VERSION}_${ARCH}.deb"
    DEB_SIZE=$(du -h "$DEB_FILE" | cut -f1)
    echo ""
    echo "✓ Package créé avec succès!"
    echo "  Fichier: $BUILD_DIR/$DEB_FILE"
    echo "  Taille: $DEB_SIZE"
    echo ""
    echo "Pour installer:"
    echo "  sudo dpkg -i $BUILD_DIR/$DEB_FILE"
    echo ""
    echo "Pour vérifier le contenu:"
    echo "  dpkg -c $BUILD_DIR/$DEB_FILE"
    echo "  dpkg -I $BUILD_DIR/$DEB_FILE"
else
    echo "ERREUR: Échec création package"
    exit 1
fi
