# License Secret Agent

Agent systemd qui protège le secret de déchiffrement des licences et expose une IPC locale.

## Démarrage rapide

```bash
# 1. Compiler
./examples/build-all.sh

# 2. Installer sur les VMs (après copie des binaires)
sudo ./examples/install-agent.sh
sudo ./examples/install-license-server.sh
sudo ./examples/install-client-app.sh

# 3. Configurer
sudo nano /etc/license-agent/config.toml

# 4. Démarrer l'agent
sudo systemctl start license-agent
```

Configuration minimale : `docs/CONFIGURATION.md`.

Notes :
- Le serveur exemple supporte **HTTP par défaut** et **HTTPS** si `--tls-cert-path`/`--tls-key-path`.
- Endpoints : `POST /api/v1/rotate-secret` et `POST /api/v1/generate-license`.
- Les clés client sont prévues dans `/etc/licence-agent/` (configurable dans `config.toml`).
- Script permissions : `sudo ./examples/fix-all-permissions-complete.sh`.
- TLS serveur : `./examples/generate-server-tls.sh /etc/license-server`
