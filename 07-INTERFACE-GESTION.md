# Interface de Gestion

## 1. Rôle et Objectifs

### 1.1 Rôle Principal

L'interface de gestion permet aux administrateurs de :
- **Visualiser** l'état du système de licences
- **Contrôler** certaines opérations (rotation, invalidation)
- **Consulter** les logs d'audit
- **Diagnostiquer** les problèmes

### 1.2 Objectifs de Sécurité

1. **Authentification forte** : Certificats ou tokens sécurisés
2. **Autorisation granulaire** : Rôles (lecture / admin)
3. **Audit complet** : Toutes les actions tracées
4. **Isolation** : Pas d'exposition du secret
5. **Minimalisme** : Surface d'attaque réduite

### 1.3 Principes de Conception

- **Principe du moindre privilège** : Accès minimal nécessaire
- **Défense en profondeur** : Plusieurs couches de sécurité
- **Non-répudiation** : Toutes actions signées et tracées
- **Simplicité** : Interface intuitive et claire

## 2. Options d'Implémentation

### 2.1 Option A : CLI Sécurisée (RECOMMANDÉE)

**Avantages** :
- ✅ Simplicité de déploiement
- ✅ Pas de surface d'attaque réseau
- ✅ Authentification via système Linux (PAM, certificats)
- ✅ Pas de dépendances supplémentaires
- ✅ Intégration naturelle avec scripts

**Inconvénients** :
- ❌ Moins convivial que UI web
- ❌ Nécessite accès SSH/local

**Architecture** :
```
CLI (license-agent-cli)
    ↓
Unix Domain Socket (authentifié)
    ↓
Secret Agent
```

**Exemple d'utilisation** :
```bash
# Authentification par certificat
license-agent-cli --cert /path/to/admin.crt --key /path/to/admin.key status

# Authentification par token
license-agent-cli --token $(cat /etc/license-agent/admin.token) status

# Actions
license-agent-cli --cert admin.crt rotate-secret
license-agent-cli --cert admin.crt invalidate-secret --version 5
```

### 2.2 Option B : API REST Locale

**Avantages** :
- ✅ Intégration facile avec outils externes
- ✅ Possibilité d'interface web locale
- ✅ Standard et familier

**Inconvénients** :
- ❌ Surface d'attaque réseau plus large
- ❌ Nécessite serveur HTTP
- ❌ Configuration TLS locale

**Architecture** :
```
Client (curl, Postman, UI web)
    ↓
HTTPS localhost:8443 (TLS 1.3, certificats)
    ↓
Secret Agent (API REST)
```

**Sécurité requise** :
- TLS 1.3 obligatoire
- Certificats locaux (auto-signés ou CA interne)
- Authentification mutuelle
- Rate limiting
- Binding localhost uniquement

**Exemple d'utilisation** :
```bash
# Status
curl --cert admin.crt --key admin.key \
     https://localhost:8443/api/v1/status

# Rotation
curl --cert admin.crt --key admin.key \
     -X POST https://localhost:8443/api/v1/rotate-secret
```

### 2.3 Option C : UI Web Locale (DÉCONSEILLÉE)

**Avantages** :
- ✅ Interface graphique conviviale
- ✅ Visualisation riche

**Inconvénients** :
- ❌ Complexité sécurité accrue
- ❌ Dépendances supplémentaires (serveur web, framework)
- ❌ Surface d'attaque élargie
- ❌ Maintenance plus complexe

**Recommandation** : Éviter sauf besoin spécifique justifié.

**Si nécessaire** :
- Serveur web minimal (actix-web en Rust)
- Authentification forte (certificats)
- Pas d'exposition réseau (localhost uniquement)
- Audit complet

## 3. Fonctionnalités

### 3.1 Visualisation (Lecture)

#### 3.1.1 État des Secrets

**Informations affichées** :
```
Secret ACTIF:
  Version: 5
  État: ACTIF
  Valide depuis: 2024-01-15 10:30:00 UTC
  Valide jusqu'à: 2024-01-16 10:30:00 UTC
  Prochaine rotation: 2024-01-16 09:30:00 UTC

Secrets GRACE:
  Version 4:
    État: GRACE
    Valide jusqu'à: 2024-01-15 10:30:00 UTC
    Grace jusqu'à: 2024-01-22 10:30:00 UTC
    Temps restant: 6 jours 23 heures

  Version 3:
    État: GRACE
    Valide jusqu'à: 2024-01-14 10:30:00 UTC
    Grace jusqu'à: 2024-01-21 10:30:00 UTC
    Temps restant: 5 jours 23 heures
```

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt status
```

**API REST** :
```bash
GET /api/v1/status
```

**Réponse JSON** :
```json
{
    "active_secret": {
        "version": 5,
        "state": "ACTIF",
        "valid_from": "2024-01-15T10:30:00Z",
        "valid_until": "2024-01-16T10:30:00Z",
        "next_rotation": "2024-01-16T09:30:00Z"
    },
    "grace_secrets": [
        {
            "version": 4,
            "state": "GRACE",
            "valid_until": "2024-01-15T10:30:00Z",
            "grace_until": "2024-01-22T10:30:00Z",
            "remaining_seconds": 604800
        }
    ],
    "invalidated_secrets": []
}
```

#### 3.1.2 État TPM

**Informations affichées** :
```
TPM Status:
  Disponible: Oui
  Version: 2.0
  Fabricant: Infineon
  Firmware: 7.85
  Clés chargées: 3
  Espace NV utilisé: 45% (9/20 index)
```

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt tpm-status
```

**API REST** :
```bash
GET /api/v1/tpm/status
```

#### 3.1.3 État Licence

**Informations affichées** :
```
License Status:
  Mode dégradé: Non
  Dernière validation: 2024-01-15 11:00:00 UTC
  Validations totales: 1234
  Validations réussies: 1230
  Validations échouées: 4
  Dernière erreur: Aucune
```

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt license-status
```

**API REST** :
```bash
GET /api/v1/license/status
```

#### 3.1.4 Métriques Système

**Informations affichées** :
```
Metrics:
  Uptime: 5 jours 12 heures
  Mémoire utilisée: 45 MB
  CPU moyen: 2.3%
  Rotations totales: 150
  Rotations réussies: 148
  Rotations échouées: 2
  Mode dégradé activations: 3
  Durée mode dégradé totale: 2 heures 15 minutes
```

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt metrics
```

**API REST** :
```bash
GET /api/v1/metrics
```

### 3.2 Actions (Admin)

#### 3.2.1 Forcer Rotation

**Description** : Déclencher manuellement une rotation du secret.

**Sécurité** :
- Rôle admin requis
- Authentification forte
- Confirmation requise (optionnel)

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt rotate-secret --force
```

**API REST** :
```bash
POST /api/v1/rotate-secret
{
    "force": true
}
```

**Réponse** :
```json
{
    "success": true,
    "old_version": 5,
    "new_version": 6,
    "rotation_time": "2024-01-15T12:00:00Z"
}
```

**Audit** :
```json
{
    "timestamp": "2024-01-15T12:00:00Z",
    "event": "rotation_forced",
    "admin": "admin@example.com",
    "old_version": 5,
    "new_version": 6
}
```

#### 3.2.2 Invalider un Secret

**Description** : Invalider immédiatement un secret (ACTIF ou GRACE).

**Sécurité** :
- Rôle admin requis
- Confirmation obligatoire (surtout pour secret ACTIF)
- Vérification impact (si secret ACTIF, alerte)

**Commande CLI** :
```bash
# Invalider secret GRACE
license-agent-cli --cert admin.crt invalidate-secret --version 4

# Invalider secret ACTIF (confirmation requise)
license-agent-cli --cert admin.crt invalidate-secret --version 5 --confirm
```

**API REST** :
```bash
POST /api/v1/secrets/{version}/invalidate
{
    "confirm": true,
    "reason": "Security incident"
}
```

**Vérifications** :
- Si secret ACTIF : Alerte + confirmation
- Si dernier secret : Passage état ABSENT
- Impact sur validations : Log détaillé

**Audit** :
```json
{
    "timestamp": "2024-01-15T12:00:00Z",
    "event": "secret_invalidated",
    "admin": "admin@example.com",
    "version": 4,
    "reason": "Security incident",
    "previous_state": "GRACE"
}
```

#### 3.2.3 Activer/Désactiver Mode Dégradé

**Description** : Contrôler manuellement le mode dégradé.

**Sécurité** :
- Rôle admin requis
- Justification requise pour activation
- Vérification serveur pour désactivation

**Commande CLI** :
```bash
# Activer (justification requise)
license-agent-cli --cert admin.crt degraded-mode --enable --reason "Network maintenance"

# Désactiver
license-agent-cli --cert admin.crt degraded-mode --disable
```

**API REST** :
```bash
POST /api/v1/degraded-mode
{
    "enabled": true,
    "reason": "Network maintenance"
}
```

**Audit** :
```json
{
    "timestamp": "2024-01-15T12:00:00Z",
    "event": "degraded_mode_manual_activation",
    "admin": "admin@example.com",
    "reason": "Network maintenance"
}
```

#### 3.2.4 Réinitialisation Complète

**Description** : Réinitialiser complètement le système (tous secrets).

**Sécurité** :
- Rôle admin requis
- Confirmation multiple requise
- Backup automatique avant réinitialisation
- Récupération depuis serveur obligatoire

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt reset --confirm --confirm-again
```

**API REST** :
```bash
POST /api/v1/reset
{
    "confirm": true,
    "confirm_again": true
}
```

**Processus** :
1. Backup état actuel
2. Invalidation tous secrets
3. Nettoyage TPM (optionnel)
4. État ABSENT
5. Récupération depuis serveur

### 3.3 Consultation Logs

#### 3.3.1 Logs d'Audit

**Description** : Consultation des logs d'audit.

**Filtres disponibles** :
- Par date (depuis, jusqu'à)
- Par événement (rotation, validation, etc.)
- Par niveau (info, warning, error)
- Par secret version

**Commande CLI** :
```bash
# Derniers logs
license-agent-cli --cert admin.crt logs --tail 100

# Logs filtrés
license-agent-cli --cert admin.crt logs \
    --since "2024-01-15 00:00:00" \
    --event "rotation" \
    --level "warning"
```

**API REST** :
```bash
GET /api/v1/audit/logs?since=2024-01-15T00:00:00Z&event=rotation&limit=100
```

**Format réponse** :
```json
{
    "logs": [
        {
            "timestamp": "2024-01-15T10:30:00Z",
            "event": "rotation_succeeded",
            "level": "info",
            "data": {...}
        }
    ],
    "total": 150,
    "limit": 100
}
```

#### 3.3.2 Export Logs

**Description** : Export des logs pour analyse externe.

**Commande CLI** :
```bash
license-agent-cli --cert admin.crt logs --export --format json --output audit.json
```

**API REST** :
```bash
GET /api/v1/audit/export?format=json&since=2024-01-01T00:00:00Z
```

## 4. Authentification et Autorisation

### 4.1 Authentification par Certificat

**Génération certificat admin** :

```bash
# Génération clé privée
openssl genrsa -out admin.key 2048

# Génération CSR
openssl req -new -key admin.key -out admin.csr \
    -subj "/CN=admin@example.com/O=Admin"

# Signature par CA interne
openssl x509 -req -in admin.csr \
    -CA /etc/license-agent/ca.crt \
    -CAkey /etc/license-agent/ca.key \
    -out admin.crt -days 365
```

**Vérification côté Agent** :

```rust
fn verify_certificate(cert: &Certificate) -> Result<AdminIdentity> {
    // 1. Vérification chaîne
    if !verify_certificate_chain(cert, ca_cert) {
        return Err(AuthError::InvalidCertificate);
    }
    
    // 2. Vérification expiration
    if cert.not_after < now() {
        return Err(AuthError::CertificateExpired);
    }
    
    // 3. Vérification révocation
    if cert_revoked(cert) {
        return Err(AuthError::CertificateRevoked);
    }
    
    // 4. Extraction identité
    let cn = extract_cn(cert.subject);
    let admin = AdminIdentity::from_cn(cn);
    
    Ok(admin)
}
```

### 4.2 Authentification par Token

**Génération token** :

```bash
# Côté serveur ou script admin
token=$(echo -n "$agent_id|$user_id|$expiration" | \
    openssl dgst -hmac "$master_key" -sha256 | \
    cut -d' ' -f2)

signed_token=$(echo -n "$user_id|$expiration|$token" | base64)
```

**Vérification côté Agent** :

```rust
fn verify_token(token: &str) -> Result<AdminIdentity> {
    let (user_id, expiration, token_hash) = decode_token(token)?;
    
    // Vérification expiration
    if expiration < now() {
        return Err(AuthError::TokenExpired);
    }
    
    // Vérification hash
    let expected_hash = hmac_sha256(
        master_key,
        format!("{}|{}|{}", agent_id, user_id, expiration)
    );
    
    if !constant_time_compare(token_hash, expected_hash) {
        return Err(AuthError::InvalidToken);
    }
    
    Ok(AdminIdentity::from_user_id(user_id))
}
```

### 4.3 Rôles et Permissions

**Rôles** :

```rust
enum Role {
    Reader,  // Lecture seule
    Admin,   // Toutes actions
}

struct Permissions {
    can_view_status: bool,
    can_view_logs: bool,
    can_rotate_secret: bool,
    can_invalidate_secret: bool,
    can_manage_degraded_mode: bool,
    can_reset: bool,
}
```

**Mapping** :
- `Reader` : Visualisation uniquement
- `Admin` : Toutes actions

**Configuration** :
```toml
[management]
roles = {
    "admin@example.com" = "Admin",
    "operator@example.com" = "Reader"
}
```

## 5. Sécurité

### 5.1 Rate Limiting

**Protection** : Limiter le nombre de requêtes par admin.

**Configuration** :
```toml
[management]
rate_limit_requests_per_minute = 60
rate_limit_burst = 10
```

**Implémentation** :
- Token bucket algorithm
- Par admin (identifié par certificat/token)
- Blocage temporaire si dépassement

### 5.2 Audit Complet

**Toutes actions tracées** :
- Qui (admin identité)
- Quoi (action)
- Quand (timestamp)
- Résultat (succès/échec)
- Contexte (paramètres, impact)

**Logs immutables** :
- Append-only
- Horodatage cryptographique (optionnel)
- Pas de modification possible

### 5.3 Isolation

**Règles** :
- Interface de gestion ne peut pas accéder au secret en clair
- Actions validées cryptographiquement
- Pas d'exposition du secret dans réponses

**Vérifications** :
- Aucun secret en clair dans logs
- Aucun secret dans réponses API
- Validation cryptographique de toutes actions

## 6. Exemples d'Utilisation

### 6.1 Vérification État Quotidienne

```bash
#!/bin/bash
# Script de monitoring quotidien

status=$(license-agent-cli --cert admin.crt status --json)

# Vérifier mode dégradé
if echo "$status" | jq -e '.degraded_mode_active == true' > /dev/null; then
    echo "ALERTE: Mode dégradé actif"
    send_alert "Mode dégradé activé"
fi

# Vérifier expiration secret
valid_until=$(echo "$status" | jq -r '.active_secret.valid_until')
if [ "$(date -d "$valid_until" +%s)" -lt "$(date -d "+1 day" +%s)" ]; then
    echo "ALERTE: Secret expire bientôt"
    send_alert "Secret expire dans moins de 24h"
fi
```

### 6.2 Rotation Manuelle Planifiée

```bash
#!/bin/bash
# Rotation avant maintenance

# 1. Vérifier état
license-agent-cli --cert admin.crt status

# 2. Forcer rotation
license-agent-cli --cert admin.crt rotate-secret --force

# 3. Vérifier succès
if [ $? -eq 0 ]; then
    echo "Rotation réussie"
else
    echo "Échec rotation"
    exit 1
fi
```

### 6.3 Diagnostic Problème

```bash
#!/bin/bash
# Diagnostic complet

echo "=== État Secrets ==="
license-agent-cli --cert admin.crt status

echo "=== État TPM ==="
license-agent-cli --cert admin.crt tpm-status

echo "=== Derniers Logs ==="
license-agent-cli --cert admin.crt logs --tail 50

echo "=== Métriques ==="
license-agent-cli --cert admin.crt metrics
```

## 7. Conclusion

L'interface de gestion fournit :

1. **Visibilité** : État complet du système
2. **Contrôle** : Actions admin sécurisées
3. **Traçabilité** : Logs d'audit complets
4. **Sécurité** : Authentification forte et audit
5. **Simplicité** : Interface claire et intuitive

L'option CLI sécurisée est recommandée pour la simplicité et la sécurité, avec possibilité d'ajouter une API REST si nécessaire pour l'intégration.
