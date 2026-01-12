# Guide de Packaging

Ce guide explique comment créer des packages d'installation pour License Secret Agent.

## Types de Packages

Le projet supporte la création de plusieurs types de packages :

1. **Package .deb** (Debian/Ubuntu) - Recommandé
2. **Tarball** (Distribution générique)
3. **Package .rpm** (RedHat/CentOS/Fedora)

## Prérequis

### Pour tous les packages
- Rust 1.70+ installé
- Projet compilé avec `cargo build --release`

### Pour package .deb
```bash
sudo apt-get install dpkg-dev
```

### Pour package .rpm
```bash
sudo yum install rpm-build rpmdevtools
# ou
sudo dnf install rpm-build rpmdevtools
```

## Construction des Packages

### 1. Package .deb (Debian/Ubuntu)

```bash
# Construire le package
./scripts/build-package.sh [VERSION] [ARCH]

# Exemples
./scripts/build-package.sh 0.1.0 amd64
./scripts/build-package.sh 0.1.0 arm64

# Le package sera créé dans target/package/
# Installer avec:
sudo dpkg -i target/package/license-secret-agent_0.1.0_amd64.deb
```

**Fichiers créés :**
- `target/package/license-secret-agent_0.1.0_amd64.deb`

**Installation :**
```bash
sudo dpkg -i license-secret-agent_0.1.0_amd64.deb

# Si dépendances manquantes:
sudo apt-get install -f
```

**Vérification :**
```bash
# Contenu du package
dpkg -c license-secret-agent_0.1.0_amd64.deb

# Informations du package
dpkg -I license-secret-agent_0.1.0_amd64.deb

# Vérifier installation
dpkg -l | grep license-secret-agent
```

### 2. Tarball (Distribution générique)

```bash
# Construire le tarball
./scripts/build-tarball.sh [VERSION] [ARCH]

# Exemples
./scripts/build-tarball.sh 0.1.0 x86_64
./scripts/build-tarball.sh 0.1.0 aarch64

# Le tarball sera créé dans target/tarball/
```

**Fichiers créés :**
- `target/tarball/license-secret-agent-0.1.0-x86_64.tar.gz`

**Utilisation :**
```bash
# Extraire
tar -xzf license-secret-agent-0.1.0-x86_64.tar.gz
cd license-secret-agent-0.1.0-x86_64

# Installer avec script
sudo ./scripts/install.sh

# OU installation manuelle (voir README.txt dans le tarball)
```

### 3. Package .rpm (RedHat/CentOS/Fedora)

```bash
# Construire le package RPM
./scripts/build-rpm.sh [VERSION] [ARCH]

# Exemples
./scripts/build-rpm.sh 0.1.0 x86_64
./scripts/build-rpm.sh 0.1.0 aarch64

# Le package sera créé dans ~/rpmbuild/RPMS/
```

**Fichiers créés :**
- `~/rpmbuild/RPMS/x86_64/license-secret-agent-0.1.0-1.x86_64.rpm`

**Installation :**
```bash
sudo rpm -ivh license-secret-agent-0.1.0-1.x86_64.rpm

# ou avec yum/dnf
sudo yum install license-secret-agent-0.1.0-1.x86_64.rpm
```

## Structure des Packages

### Package .deb

```
license-secret-agent_0.1.0_amd64.deb
├── usr/bin/
│   ├── license-agent
│   └── license-agent-cli
├── usr/lib/systemd/system/
│   └── license-agent.service
├── etc/license-agent/
│   └── config.toml.example
├── usr/share/license-agent/
│   └── config.toml.example
└── DEBIAN/
    ├── control
    ├── postinst
    ├── prerm
    └── postrm
```

### Tarball

```
license-secret-agent-0.1.0-x86_64.tar.gz
├── bin/
│   ├── license-agent
│   └── license-agent-cli
├── etc/
│   └── config.toml.example
├── systemd/
│   └── license-agent.service
├── scripts/
│   └── install.sh
└── README.txt
```

## Scripts Post-Installation

### postinst (après installation .deb)
- Crée l'utilisateur `license-agent`
- Crée les répertoires nécessaires
- Configure les permissions
- Copie la configuration exemple si absente
- Recharge systemd

### prerm (avant suppression .deb)
- Arrête le service
- Désactive le service au démarrage

### postrm (après suppression .deb)
- Recharge systemd
- Conserve les données (configuration, logs)

## Dépendances

Les packages déclarent les dépendances suivantes :

- **libssl3** (>= 3.0.0) ou **openssl** (>= 3.0.0)
- **systemd**
- **tpm2-tss** (>= 3.0.0) - Recommandé mais optionnel

## Versioning

Le versioning suit le format : `MAJOR.MINOR.PATCH`

- **MAJOR** : Changements incompatibles
- **MINOR** : Nouvelles fonctionnalités compatibles
- **PATCH** : Corrections de bugs

## Distribution

### Pour distribution interne

1. Construire le package
2. Héberger sur un serveur de packages (APT/YUM)
3. Configurer les dépôts sur les machines cibles

### Pour distribution publique

1. Construire le package
2. Tester sur machines propres
3. Signer le package (optionnel mais recommandé)
4. Publier sur GitHub Releases ou autre plateforme

## Signing des Packages (Optionnel)

### Signer un package .deb

```bash
# Générer clé GPG (si pas déjà fait)
gpg --gen-key

# Signer le package
dpkg-sig --sign builder license-secret-agent_0.1.0_amd64.deb

# Vérifier signature
dpkg-sig --verify license-secret-agent_0.1.0_amd64.deb
```

### Signer un package .rpm

```bash
# Signer avec GPG
rpm --addsign license-secret-agent-0.1.0-1.x86_64.rpm

# Vérifier signature
rpm --checksig license-secret-agent-0.1.0-1.x86_64.rpm
```

## Dépannage

### Erreur de compilation
- Vérifier que Rust est installé : `rustc --version`
- Vérifier dépendances système : `cargo build --release`

### Erreur de construction package .deb
- Vérifier que `dpkg-deb` est installé
- Vérifier permissions des scripts dans `deb/`

### Erreur de construction package .rpm
- Vérifier que `rpmbuild` est installé
- Vérifier structure `~/rpmbuild/`

### Problèmes d'installation
- Vérifier dépendances : `dpkg -I package.deb` ou `rpm -qpR package.rpm`
- Installer dépendances manquantes
- Vérifier logs : `journalctl -u license-agent`

## Automatisation CI/CD

Exemple pour GitHub Actions :

```yaml
name: Build Packages

on:
  release:
    types: [created]

jobs:
  build-deb:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: ./scripts/build-package.sh ${{ github.event.release.tag_name }} amd64
      - uses: actions/upload-artifact@v3
        with:
          name: deb-package
          path: target/package/*.deb
```

## Notes

- Les packages incluent les binaires compilés (release)
- La configuration doit être modifiée après installation
- Les certificats doivent être installés séparément
- Le service n'est pas démarré automatiquement après installation
