# Guide de Configuration

## Fichier de Configuration

Le fichier de configuration principal est `/etc/license-agent/config.toml`.

## Sections de Configuration

### [server]

Configuration du serveur de licences distant.

```toml
[server]
url = "https://license-server.example.com"  # URL du serveur
cert_pin = "sha256:..."                     # Pin du certificat serveur (SHA-256)
client_cert = "/etc/license-agent/client.pem"  # Certificat client
client_key = "/etc/license-agent/client.key"   # Clé privée client
timeout_seconds = 30                        # Timeout requêtes (optionnel)
```

**cert_pin** : Hash SHA-256 du certificat serveur pour pinning. Obtenir avec :
```bash
openssl s_client -connect license-server.example.com:443 -servername license-server.example.com < /dev/null 2>/dev/null | openssl x509 -fingerprint -sha256 -noout | cut -d'=' -f2
```

### [agent]

Configuration de l'agent local.

```toml
[agent]
id = "pos-001"                              # Identifiant unique (obligatoire)
rotation_interval = 86400                   # Intervalle rotation (secondes, défaut: 86400 = 24h)
grace_period = 604800                       # Période de grâce (secondes, défaut: 604800 = 7j)
rotation_threshold_seconds = 3600          # Seuil déclenchement rotation (optionnel, défaut: 3600)
```

**id** : Doit être unique pour chaque POS. Utilisé pour identifier l'agent auprès du serveur.

### [tpm]

Configuration TPM.

```toml
[tpm]
enabled = true                              # Activer TPM (défaut: true)
fallback_encrypted_storage = "/var/lib/license-agent/secret.enc"  # Fallback si TPM indisponible
```

Si `enabled = false` ou TPM indisponible, le fallback chiffrement logiciel sera utilisé (moins sécurisé).

### [management]

Configuration interface de gestion.

```toml
[management]
allowed_uids = [1000, 1001]                 # UIDs autorisés pour CLI (vide = tous)
ipc_socket_path = "/var/run/license-agent.sock"  # Chemin socket IPC (optionnel)
api_port = null                            # Port API REST (null = désactivé, optionnel)
rate_limit_requests_per_minute = 60         # Limite requêtes/minute (optionnel)
```

### [degraded_mode]

Configuration mode dégradé.

```toml
[degraded_mode]
enabled = true                              # Activer mode dégradé (défaut: true)
grace_period_days = 7                       # Durée grace period (jours, défaut: 7)
retry_interval_seconds = 300                # Intervalle retry rotation (secondes, défaut: 300)
auto_deactivate_on_reconnect = true        # Désactivation auto sur reconnexion (défaut: true)
alert_thresholds_hours = [24, 72, 144]      # Seuils alertes (heures, défaut: [24, 72, 144])
```

## Exemple Complet

```toml
[server]
url = "https://license-server.example.com"
cert_pin = "sha256:ABCDEF1234567890..."
client_cert = "/etc/license-agent/client.pem"
client_key = "/etc/license-agent/client.key"
timeout_seconds = 30

[agent]
id = "pos-001"
rotation_interval = 86400
grace_period = 604800
rotation_threshold_seconds = 3600

[tpm]
enabled = true
fallback_encrypted_storage = "/var/lib/license-agent/secret.enc"

[management]
allowed_uids = [1000, 1001]
ipc_socket_path = "/var/run/license-agent.sock"
api_port = null
rate_limit_requests_per_minute = 60

[degraded_mode]
enabled = true
grace_period_days = 7
retry_interval_seconds = 300
auto_deactivate_on_reconnect = true
alert_thresholds_hours = [24, 72, 144]
```

## Validation

Le service valide la configuration au démarrage. Erreurs communes :

- URL serveur doit commencer par `https://`
- Certificats doivent exister et être lisibles
- `rotation_interval` et `grace_period` doivent être > 0
- `agent.id` ne doit pas être vide

## Variables d'Environnement

Pour le fallback chiffrement (si TPM indisponible) :

```bash
export LICENSE_AGENT_FALLBACK_KEY="votre-clé-secrète"
```

⚠️ **ATTENTION** : Ne pas utiliser en production. Utiliser TPM 2.0.
