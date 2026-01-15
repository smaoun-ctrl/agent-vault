# Configuration

Fichier principal : `/etc/license-agent/config.toml`.

## Exemple minimal

```toml
[server]
url = "https://license-server.example.com"
cert_pin = ""
client_cert = "/etc/licence-agent/client.pem"
client_key = "/etc/licence-agent/client.key"
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
allowed_uids = [1000]
ipc_socket_path = "/var/run/license-agent.sock"
rate_limit_requests_per_minute = 60

[degraded_mode]
enabled = true
grace_period_days = 7
retry_interval_seconds = 300
auto_deactivate_on_reconnect = true
alert_thresholds_hours = [24, 72, 144]
```

## Notes

- `server.url` accepte `http://` ou `https://` (le serveur exemple est HTTP par défaut, HTTPS optionnel).
- `cert_pin` vide désactive le pinning.
- `api_port` est optionnel : omettez la clé pour désactiver l'API.
- `client_cert` doit être un certificat X.509 (pas une simple clé publique).
