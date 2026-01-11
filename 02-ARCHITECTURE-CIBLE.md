# Architecture Cible Complète

## 1. Vue d'Ensemble

```
┌─────────────────────────────────────────────────────────────────┐
│                         POS Application                          │
│  (Application métier utilisant les licences)                    │
└───────────────────────┬─────────────────────────────────────────┘
                        │ Unix Domain Socket (SO_PEERCRED)
                        │ Requêtes: validate_license(token)
                        │ Réponses: {valid: bool, metadata: {...}}
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    License Secret Agent                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Core Engine                                              │  │
│  │  - Gestion du cycle de vie des secrets                   │  │
│  │  - Rotation automatique                                  │  │
│  │  - Validation des licences                               │  │
│  │  - Mode dégradé                                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Secret Manager                                          │  │
│  │  - Stockage TPM 2.0                                      │  │
│  │  - Chiffrement mémoire                                   │  │
│  │  - Zeroization                                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Rotation Manager                                        │  │
│  │  - Coordination rotation                                 │  │
│  │  - Coexistence multi-secrets                             │  │
│  │  - Validation serveur                                    │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Audit Logger                                            │  │
│  │  - Logs immutables                                       │  │
│  │  - Métriques                                             │  │
│  └──────────────────────────────────────────────────────────┘  │
└───────────┬───────────────────────────────┬─────────────────────┘
            │                               │
            │ HTTPS (TLS 1.3)               │ Unix Domain Socket
            │ Authentification mutuelle     │ Authentification forte
            │                               │
            ▼                               ▼
┌───────────────────────────┐   ┌──────────────────────────────┐
│   Serveur de Licences     │   │  Interface de Gestion       │
│   (Internet)              │   │  - CLI sécurisée            │
│                           │   │  - API locale (optionnel)   │
│  - Génération licences    │   │  - Visualisation état       │
│  - Rotation secrets       │   │  - Actions contrôlées        │
│  - Validation             │   │  - Consultation logs        │
└───────────────────────────┘   └──────────────────────────────┘
            │
            │ TPM 2.0 (matériel)
            ▼
┌───────────────────────────┐
│   TPM 2.0 Hardware        │
│  - Clé non exportable     │
│  - Stockage sécurisé      │
│  - Opérations cryptographiques
└───────────────────────────┘
```

## 2. Composants Principaux

### 2.1 License Secret Agent (Service Systemd)

**Rôle** : Composant central gérant le cycle de vie des secrets et la validation des licences.

**Caractéristiques** :
- **Langage** : Rust (sécurité mémoire, performance)
- **Type** : Service systemd (daemon)
- **Utilisateur** : `license-agent` (UID dédié, non-privilégié)
- **Capabilities** : `CAP_SYS_ADMIN` (pour TPM), `CAP_NET_BIND_SERVICE` (si API locale)
- **Isolation** :
  - Namespace utilisateur
  - Seccomp BPF (filtrage syscalls)
  - AppArmor/SELinux profile
  - Pas de réseau direct (sauf vers serveur de licences)

**Modules internes** :
1. **Core Engine** : Orchestration générale
2. **Secret Manager** : Gestion TPM et mémoire
3. **Rotation Manager** : Logique de rotation
4. **License Validator** : Déchiffrement et validation
5. **Network Client** : Communication serveur
6. **IPC Server** : Unix Domain Socket
7. **Audit Logger** : Traçabilité
8. **Management Interface** : Exposition interface de gestion

### 2.2 POS Application

**Rôle** : Application métier consommant les licences.

**Caractéristiques** :
- **Communication** : Unix Domain Socket (lecture seule)
- **Permissions** : UID de l'application POS
- **API** : `validate_license(token: &[u8]) -> Result<LicenseInfo>`
- **Isolation** : Pas d'accès direct au secret

**Flux d'utilisation** :
1. Application reçoit un token de licence (chiffré)
2. Envoie requête au Secret Agent via Unix Socket
3. Reçoit validation + métadonnées (sans le secret)
4. Utilise le résultat pour autoriser/débloquer fonctionnalités

### 2.3 Interface de Gestion

**Rôle** : Permettre la visualisation et le contrôle local du système.

**Options d'implémentation** :

#### Option A : CLI Sécurisée (RECOMMANDÉE)
- **Avantages** :
  - Simplicité de déploiement
  - Pas de surface d'attaque réseau
  - Authentification via système Linux (PAM, certificats)
- **Exemple** : `license-agent-cli --auth-cert /path/to/cert status`

#### Option B : API Locale REST
- **Avantages** : Intégration facile, interface web possible
- **Inconvénients** : Surface d'attaque plus large
- **Sécurité requise** :
  - HTTPS avec certificats locaux
  - Authentification mutuelle
  - Rate limiting
  - Binding localhost uniquement

#### Option C : UI Web Locale (DÉCONSEILLÉE)
- **Risques** : Complexité sécurité, dépendances supplémentaires
- **Si nécessaire** : Serveur web minimal (actix-web), authentification forte, pas d'exposition réseau

**Fonctionnalités** :
- **Visualisation** :
  - État des secrets (actif, grace, dates)
  - État TPM
  - État licence
  - Métriques système
- **Actions** :
  - Forcer rotation
  - Invalider secret
  - Activer/désactiver mode dégradé (si autorisé)
  - Consultation logs d'audit
- **Sécurité** :
  - Authentification forte (certificats, tokens)
  - Rôles (lecture seule / admin)
  - Audit de toutes les actions

### 2.4 Serveur de Licences (Distant)

**Rôle** : Génération et gestion des licences et secrets.

**Caractéristiques** :
- **API HTTPS** : TLS 1.3 obligatoire
- **Authentification** : Certificats client ou tokens signés
- **Endpoints** :
  - `POST /api/v1/rotate-secret` : Rotation du secret
  - `GET /api/v1/secret-status` : État du secret actuel
  - `POST /api/v1/validate-license` : Validation licence (optionnel)

**Sécurité** :
- Certificats serveur vérifiés (pinning)
- Authentification mutuelle
- Rate limiting
- Validation cryptographique des requêtes

### 2.5 TPM 2.0 Hardware

**Rôle** : Stockage sécurisé du secret (matériel).

**Utilisation** :
- **Clé persistante** : RSA 2048 ou ECC P-256 (non exportable)
- **NV Index** : Stockage métadonnées (versions, dates)
- **Opérations** :
  - Chiffrement/déchiffrement du secret
  - Génération de nonces
  - Vérification d'intégrité

**Fallback** : Si TPM indisponible, chiffrement disque (AES-256-GCM) avec clé dérivée (PBKDF2) stockée séparément.

## 3. Flux de Communication

### 3.1 Communication POS Application ↔ Secret Agent

**Protocole** : Unix Domain Socket avec contrôle d'identité

**Sécurité** :
- `SO_PEERCRED` : Vérification UID/GID du client
- Whitelist des UIDs autorisés
- Pas de transmission du secret dans les réponses

**Messages** :
```rust
// Requête
struct ValidateLicenseRequest {
    license_token: Vec<u8>,  // Token chiffré
    nonce: [u8; 16],         // Nonce pour éviter replay
}

// Réponse
struct ValidateLicenseResponse {
    valid: bool,
    expires_at: Option<DateTime<Utc>>,
    features: Vec<String>,   // Fonctionnalités autorisées
    metadata: HashMap<String, String>,
}
```

### 3.2 Communication Secret Agent ↔ Serveur Distant

**Protocole** : HTTPS (TLS 1.3)

**Sécurité** :
- Certificat serveur vérifié (pinning)
- Certificat client pour authentification
- Chiffrement bout-en-bout

**Messages** :
```rust
// Requête rotation
struct RotateSecretRequest {
    current_secret_version: u64,
    agent_id: String,        // Identifiant unique POS
    timestamp: DateTime<Utc>,
    signature: Vec<u8>,      // Signature de la requête
}

// Réponse rotation
struct RotateSecretResponse {
    new_secret_encrypted: Vec<u8>,  // Chiffré avec clé publique agent
    secret_version: u64,
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    grace_until: DateTime<Utc>,     // Date limite mode dégradé
    signature: Vec<u8>,              // Signature serveur
}
```

### 3.3 Communication Interface de Gestion ↔ Secret Agent

**Protocole** : Unix Domain Socket (CLI) ou HTTPS local (API)

**Sécurité** :
- Authentification forte (certificats, tokens)
- Vérification des permissions
- Audit de toutes les actions

**Messages** :
```rust
// Requête statut
struct StatusRequest {
    auth_token: Vec<u8>,
}

// Réponse statut
struct StatusResponse {
    active_secret: SecretInfo,
    grace_secrets: Vec<SecretInfo>,
    tpm_status: TpmStatus,
    license_status: LicenseStatus,
    next_rotation: Option<DateTime<Utc>>,
}
```

## 4. Stockage des Données

### 4.1 Secret (TPM 2.0)

**Stockage** : Clé TPM non exportable

**Structure** :
- Le secret réel est chiffré avec la clé TPM
- Métadonnées dans NV Index TPM :
  - Version du secret
  - Date de création
  - Date d'expiration
  - État (ACTIF, GRACE, INVALIDÉ)

**Sécurité** :
- Clé TPM avec politique d'authentification
- Pas d'export possible
- Zeroization après usage en mémoire

### 4.2 Configuration

**Fichier** : `/etc/license-agent/config.toml` (permissions 600, root:root)

**Contenu** :
```toml
[server]
url = "https://license-server.example.com"
cert_pin = "sha256:..."  # Pin du certificat serveur
client_cert = "/etc/license-agent/client.pem"
client_key = "/etc/license-agent/client.key"

[agent]
id = "pos-001"  # Identifiant unique
rotation_interval = 86400  # 24h en secondes
grace_period = 604800      # 7 jours en secondes

[tpm]
enabled = true
fallback_encrypted_storage = "/var/lib/license-agent/secret.enc"

[management]
allowed_uids = [1000, 1001]  # UIDs autorisés pour interface
```

### 4.3 Logs d'Audit

**Fichier** : `/var/log/license-agent/audit.log` (append-only, permissions 640)

**Format** : JSON structuré avec horodatage

**Contenu** :
- Toutes les validations de licence
- Toutes les rotations
- Toutes les actions de gestion
- Erreurs et alertes
- Changements d'état

### 4.4 État Runtime

**Fichier** : `/var/lib/license-agent/state.json` (permissions 600)

**Contenu** :
- Liste des secrets actifs (sans les secrets eux-mêmes)
- Métadonnées de rotation
- État du mode dégradé
- Dernière connexion serveur

## 5. Isolation et Sécurité Système

### 5.1 Utilisateur Dédié

```bash
# Création utilisateur
useradd -r -s /bin/false -d /var/lib/license-agent license-agent
```

**Permissions** :
- Pas de shell
- Pas de login
- Accès limité aux fichiers nécessaires

### 5.2 Capabilities Linux

```bash
# Capabilities minimales
setcap cap_sys_admin+ep /usr/bin/license-agent
```

**Justification** :
- `CAP_SYS_ADMIN` : Accès TPM (via /dev/tpm*)
- Pas d'autres capabilities nécessaires

### 5.3 Seccomp BPF

**Filtrage syscalls** : Autoriser uniquement les syscalls nécessaires :
- `read`, `write`, `open`, `close`
- `socket`, `bind`, `listen`, `accept`
- `epoll_*`
- `clock_gettime`
- `getuid`, `getgid`
- Appels TPM spécifiques

**Blocage** :
- `ptrace` (anti-debugging)
- `execve` (pas d'exécution arbitraire)
- `mount` (pas de manipulation filesystem)

### 5.4 Namespaces

**User namespace** : Isolation UID/GID
**PID namespace** : Isolation processus
**Network namespace** : Contrôle réseau (optionnel)

### 5.5 AppArmor/SELinux

**Profile AppArmor** :
```
/usr/bin/license-agent {
    /dev/tpm* rw,
    /etc/license-agent/** r,
    /var/lib/license-agent/** rw,
    /var/log/license-agent/** w,
    network,
    deny /proc/*/mem r,
    deny ptrace,
}
```

## 6. Déploiement

### 6.1 Service Systemd

**Fichier** : `/etc/systemd/system/license-agent.service`

```ini
[Unit]
Description=License Secret Agent
After=network.target tpm2-tss.service
Requires=tpm2-tss.service

[Service]
Type=notify
User=license-agent
Group=license-agent
ExecStart=/usr/bin/license-agent
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Sécurité
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/license-agent /var/log/license-agent
CapabilityBoundingSet=CAP_SYS_ADMIN
AmbientCapabilities=CAP_SYS_ADMIN

[Install]
WantedBy=multi-user.target
```

### 6.2 Structure de Fichiers

```
/etc/license-agent/
├── config.toml          # Configuration (600, root:root)
├── client.pem           # Certificat client (600, root:root)
└── client.key           # Clé privée client (600, root:root)

/var/lib/license-agent/
├── state.json           # État runtime (600, license-agent:license-agent)
└── secret.enc           # Fallback si pas de TPM (600, license-agent:license-agent)

/var/log/license-agent/
└── audit.log            # Logs d'audit (640, license-agent:license-agent)

/usr/bin/
├── license-agent        # Binaire principal (755, root:root)
└── license-agent-cli    # CLI de gestion (755, root:root)
```

## 7. Monitoring et Alertes

### 7.1 Métriques

**Exposition** : Fichier `/var/lib/license-agent/metrics.prom` (format Prometheus)

**Métriques** :
- `license_agent_secrets_active` : Nombre de secrets actifs
- `license_agent_secrets_grace` : Nombre de secrets en grace
- `license_agent_rotations_total` : Total rotations
- `license_agent_rotations_failed` : Échecs rotation
- `license_agent_validations_total` : Total validations
- `license_agent_validations_failed` : Échecs validation
- `license_agent_degraded_mode` : Mode dégradé actif (0/1)
- `license_agent_tpm_available` : TPM disponible (0/1)
- `license_agent_last_rotation` : Timestamp dernière rotation

### 7.2 Alertes

**Conditions d'alerte** :
- Échec de rotation > 3 tentatives
- Mode dégradé actif > 24h
- TPM indisponible
- Secret expiré sans remplacement
- Tentative d'accès non autorisée

**Actions** :
- Logs d'audit
- Notification système (systemd journal)
- Optionnel : Webhook externe

## 8. Récupération et Maintenance

### 8.1 Récupération après Panne

**Scénarios** :
1. **Perte de secret** : Réinitialisation complète, nouvelle clé TPM
2. **Corruption état** : Restauration depuis backup (si disponible)
3. **TPM défaillant** : Bascule vers fallback chiffrement disque

### 8.2 Mise à Jour

**Processus** :
1. Arrêt gracieux du service
2. Sauvegarde état et configuration
3. Mise à jour binaire
4. Vérification intégrité (signature)
5. Redémarrage service
6. Vérification fonctionnement

**Rollback** : Conservation version précédente, restauration possible

## 9. Tests et Validation

### 9.1 Tests Unitaires

- Gestion des secrets
- Rotation
- Validation licences
- Mode dégradé

### 9.2 Tests d'Intégration

- Communication POS ↔ Agent
- Communication Agent ↔ Serveur
- Interface de gestion
- Récupération après panne

### 9.3 Tests de Sécurité

- Tentative d'extraction secret
- Injection de faux secrets
- Attaques par replay
- Tests de résilience

## 10. Conclusion Architecture

Cette architecture fournit :

1. **Sécurité** : Protection multi-couches du secret
2. **Disponibilité** : Mode dégradé + rotation sans coupure
3. **Traçabilité** : Logs d'audit complets
4. **Maintenabilité** : Interface de gestion et monitoring
5. **Robustesse** : Gestion d'erreurs et récupération

L'architecture est modulaire, testable et adaptée à un déploiement industriel POS.
