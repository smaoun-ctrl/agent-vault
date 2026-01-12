#!/bin/bash
set -euo pipefail

# Script de construction d'un package RPM pour RedHat/CentOS
# Nécessite rpmbuild installé

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="${1:-0.1.0}"
ARCH="${2:-x86_64}"

echo "=== Construction package RPM License Secret Agent ==="
echo "Version: $VERSION"
echo "Architecture: $ARCH"
echo ""

# Vérifier rpmbuild
if ! command -v rpmbuild &> /dev/null; then
    echo "ERREUR: rpmbuild non trouvé. Installez avec:"
    echo "  sudo yum install rpm-build rpmdevtools"
    echo "  ou"
    echo "  sudo dnf install rpm-build rpmdevtools"
    exit 1
fi

# 1. Compiler le projet
echo "1. Compilation du projet..."
cd "$PROJECT_ROOT"
cargo build --release

if [ ! -f "target/release/license-agent" ] || [ ! -f "target/release/license-agent-cli" ]; then
    echo "ERREUR: Binaires non trouvés après compilation"
    exit 1
fi

echo "✓ Compilation réussie"

# 2. Créer structure RPM
echo "2. Création structure RPM..."
RPMBUILD_DIR="$HOME/rpmbuild"
mkdir -p "$RPMBUILD_DIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

# 3. Créer spec file
SPEC_FILE="$RPMBUILD_DIR/SPECS/license-secret-agent.spec"
cat > "$SPEC_FILE" <<EOF
Name:           license-secret-agent
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        License Secret Agent - Service de protection de licences applicatives
License:        MIT OR Apache-2.0
URL:            https://github.com/smaoun-ctrl/agent-vault
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  openssl-devel
Requires:       openssl >= 3.0.0
Requires:       systemd
Recommends:     tpm2-tss >= 3.0.0

%description
Service système sécurisé pour gérer le secret de déchiffrement des licences
applicatives avec support TPM 2.0, rotation automatique et mode dégradé.

%prep
%setup -q

%build
cargo build --release

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/lib/systemd/system
mkdir -p %{buildroot}/etc/license-agent
mkdir -p %{buildroot}/usr/share/license-agent
mkdir -p %{buildroot}/var/lib/license-agent
mkdir -p %{buildroot}/var/log/license-agent

install -m 755 target/release/license-agent %{buildroot}/usr/bin/
install -m 755 target/release/license-agent-cli %{buildroot}/usr/bin/
install -m 644 deploy/license-agent.service %{buildroot}/usr/lib/systemd/system/
install -m 644 deploy/config.toml.example %{buildroot}/usr/share/license-agent/
install -m 644 deploy/config.toml.example %{buildroot}/etc/license-agent/config.toml.example

%pre
getent group license-agent >/dev/null || groupadd -r license-agent
getent passwd license-agent >/dev/null || \
    useradd -r -g license-agent -d /var/lib/license-agent -s /sbin/nologin \
    -c "License Secret Agent" license-agent

%post
systemctl daemon-reload
if [ ! -f /etc/license-agent/config.toml ]; then
    cp /usr/share/license-agent/config.toml.example /etc/license-agent/config.toml
    chmod 600 /etc/license-agent/config.toml
fi

%preun
if [ \$1 -eq 0 ]; then
    systemctl --no-reload disable license-agent > /dev/null 2>&1 || :
    systemctl stop license-agent > /dev/null 2>&1 || :
fi

%postun
systemctl daemon-reload

%files
%defattr(-,root,root,-)
/usr/bin/license-agent
/usr/bin/license-agent-cli
/usr/lib/systemd/system/license-agent.service
/etc/license-agent/config.toml.example
/usr/share/license-agent/config.toml.example
%dir /var/lib/license-agent
%dir /var/log/license-agent

%changelog
* $(date '+%a %b %d %Y') License Agent Team <admin@example.com> - ${VERSION}-1
- Version initiale
EOF

# 4. Créer archive source
echo "3. Création archive source..."
cd "$PROJECT_ROOT"
tar -czf "$RPMBUILD_DIR/SOURCES/license-secret-agent-${VERSION}.tar.gz" \
    --exclude='target' \
    --exclude='.git' \
    --exclude='*.deb' \
    --exclude='*.rpm' \
    --transform "s,^,license-secret-agent-${VERSION}/," \
    .

# 5. Construire le RPM
echo "4. Construction RPM..."
cd "$RPMBUILD_DIR"
rpmbuild -ba SPECS/license-secret-agent.spec

RPM_FILE="$RPMBUILD_DIR/RPMS/${ARCH}/license-secret-agent-${VERSION}-1.${ARCH}.rpm"
if [ -f "$RPM_FILE" ]; then
    RPM_SIZE=$(du -h "$RPM_FILE" | cut -f1)
    echo ""
    echo "✓ Package RPM créé avec succès!"
    echo "  Fichier: $RPM_FILE"
    echo "  Taille: $RPM_SIZE"
    echo ""
    echo "Pour installer:"
    echo "  sudo rpm -ivh $RPM_FILE"
    echo "  ou"
    echo "  sudo yum install $RPM_FILE"
else
    echo "ERREUR: Échec création package RPM"
    exit 1
fi
