# Guide de Déploiement

## Prérequis

- Linux avec systemd
- TPM 2.0 (recommandé) ou fallback chiffrement logiciel
- Rust 1.70+ (pour compilation)
- Bibliothèques TPM : `tpm2-tss` (si TPM disponible)

## Installation des Dépendances

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    tpm2-tss-dev \
    tpm2-tools
```

### CentOS/RHEL

```bash
sudo yum install -y \
    gcc \
    pkgconfig \
    openssl-devel \
    tpm2-tss-devel \
    tpm2-tools
```

## Compilation

```bash
# Cloner ou naviguer vers le répertoire
cd /path/to/license-secret-agent

# Compiler en mode release
cargo build --release

# Les binaires seront dans target/release/
# - license-agent : Service principal
# - license-agent-cli : CLI de gestion
```

## Installation

```bash
# Exécuter le script d'installation (nécessite root)
sudo ./deploy/install.sh
```

Le script :
1. Crée l'utilisateur `license-agent`
2. Crée les répertoires nécessaires
3. Installe les binaires
4. Installe le service systemd
5. Configure les permissions

## Configuration

### 1. Configuration Principale

Éditer `/etc/license-agent/config.toml` :

```toml
[server]
url = "https://license-server.example.com"
cert_pin = "sha256:VOTRE_PIN_CERTIFICAT"
client_cert = "/etc/license-agent/client.pem"
client_key = "/etc/license-agent/client.key"
timeout_seconds = 30

[agent]
id = "pos-001"  # Identifiant unique par POS
rotation_interval = 86400  # 24 heures
grace_period = 604800      # 7 jours

[tpm]
enabled = true

[management]
allowed_uids = [1000, 1001]  # UIDs autorisés pour CLI
```

### 2. Certificats

Installer les certificats client :

```bash
# Copier certificat et clé
sudo cp client.pem /etc/license-agent/
sudo cp client.key /etc/license-agent/

# Permissions strictes
sudo chmod 600 /etc/license-agent/client.pem
sudo chmod 600 /etc/license-agent/client.key
sudo chown root:root /etc/license-agent/client.pem
sudo chown root:root /etc/license-agent/client.key
```

### 3. Génération Clés Agent (Première Exécution)

Si les clés n'existent pas, elles seront générées automatiquement au premier démarrage.

Pour générer manuellement :

```bash
# Le service générera les clés au démarrage si absentes
# Ou utiliser un script de génération (à créer)
```

## Démarrage

```bash
# Démarrer le service
sudo systemctl start license-agent

# Activer au démarrage
sudo systemctl enable license-agent

# Vérifier le statut
sudo systemctl status license-agent

# Consulter les logs
sudo journalctl -u license-agent -f
```

## Vérification

```bash
# Vérifier le statut via CLI
sudo license-agent-cli status

# Vérifier TPM
sudo license-agent-cli tpm-status

# Vérifier les métriques
sudo license-agent-cli metrics
```

## Dépannage

### Service ne démarre pas

```bash
# Vérifier les logs
sudo journalctl -u license-agent -n 50

# Vérifier la configuration
sudo license-agent-cli --help

# Vérifier les permissions
ls -la /etc/license-agent/
ls -la /var/lib/license-agent/
ls -la /var/log/license-agent/
```

### TPM non disponible

Si TPM n'est pas disponible, le service utilisera le fallback chiffrement logiciel.
Vérifier les logs pour confirmer.

### Problèmes de rotation

```bash
# Forcer une rotation manuelle
sudo license-agent-cli rotate --force

# Vérifier les logs de rotation
sudo journalctl -u license-agent | grep rotation
```

## Mise à Jour

```bash
# Arrêter le service
sudo systemctl stop license-agent

# Compiler nouvelle version
cargo build --release

# Réinstaller
sudo ./deploy/install.sh

# Redémarrer
sudo systemctl start license-agent
```

## Sécurité

- Vérifier régulièrement les permissions (600 pour fichiers sensibles)
- Surveiller les logs d'audit
- Maintenir le système à jour
- Configurer firewall si API REST activée
- Utiliser TPM 2.0 si disponible
