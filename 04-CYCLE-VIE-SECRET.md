# Cycle de Vie du Secret

## 1. États du Secret

Le secret peut se trouver dans l'un des états suivants :

```
ABSENT → ACTIF → GRACE → INVALIDÉ
   ↑                              ↓
   └──────────────────────────────┘
        (réinitialisation complète)
```

### 1.1 État ABSENT

**Définition** : Aucun secret n'est disponible ou valide.

**Conditions d'entrée** :
- Installation initiale du système
- Échec de rotation sans secret de repli
- Expiration de tous les secrets (actif + grace)
- Réinitialisation manuelle
- Corruption irrécupérable des données TPM

**Caractéristiques** :
- Aucune validation de licence possible
- Mode dégradé non applicable (pas de secret à utiliser)
- Service en erreur critique
- Alertes générées immédiatement

**Actions possibles** :
- Tentative de récupération depuis serveur
- Réinitialisation complète
- Récupération depuis backup (si disponible)

**Transitions** :
- `ABSENT → ACTIF` : Réception et activation d'un nouveau secret depuis serveur

**Durée attendue** : Minimale (quelques secondes à minutes)

### 1.2 État ACTIF

**Définition** : Le secret est valide et utilisé pour toutes les nouvelles validations de licence.

**Conditions d'entrée** :
- Réception d'un nouveau secret depuis serveur
- Activation après rotation réussie
- Réinitialisation après état ABSENT

**Caractéristiques** :
- Secret utilisé pour toutes les validations
- Secret stocké dans TPM (chiffré)
- Métadonnées : `valid_from ≤ now() < valid_until`
- État : `ACTIF`

**Validations** :
- Toutes les licences utilisant ce secret sont acceptées
- Licences utilisant des versions antérieures peuvent utiliser secrets en GRACE

**Transitions** :
- `ACTIF → GRACE` : Réception d'un nouveau secret (rotation)
- `ACTIF → INVALIDÉ` : Invalidation manuelle ou corruption
- `ACTIF → ABSENT` : Expiration sans remplacement (cas d'erreur)

**Durée attendue** : `rotation_interval` (ex: 24h)

### 1.3 État GRACE

**Définition** : Le secret n'est plus actif mais reste utilisable pour les licences existantes pendant une période de grâce.

**Conditions d'entrée** :
- Réception d'un nouveau secret (l'ancien passe en GRACE)
- Rotation automatique ou manuelle

**Caractéristiques** :
- Secret toujours stocké dans TPM (chiffré)
- Métadonnées : `valid_until ≤ now() < grace_until`
- État : `GRACE`
- Utilisé uniquement pour licences référençant explicitement cette version

**Validations** :
- Licences avec `version == grace_secret.version` : ACCEPTÉES
- Licences avec `version == active_secret.version` : Utilisent secret ACTIF
- Licences avec `version < grace_secret.version` : Rejetées (trop anciennes)

**Transitions** :
- `GRACE → INVALIDÉ` : Expiration de la période de grâce (`now() >= grace_until`)
- `GRACE → INVALIDÉ` : Invalidation manuelle
- `GRACE → ABSENT` : Corruption ou erreur (cas exceptionnel)

**Durée attendue** : `grace_period` (ex: 7 jours)

**Justification** :
- Permet la coexistence de licences générées avec différents secrets
- Évite les interruptions de service lors de la rotation
- Fenêtre de transition pour les licences en transit

### 1.4 État INVALIDÉ

**Définition** : Le secret n'est plus utilisable, définitivement désactivé.

**Conditions d'entrée** :
- Expiration de la période de grâce
- Invalidation manuelle via interface de gestion
- Détection de compromission
- Corruption détectée

**Caractéristiques** :
- Secret toujours dans TPM (pour audit) ou effacé (sécurité renforcée)
- Métadonnées : État `INVALIDÉ`
- Aucune validation possible avec ce secret

**Validations** :
- Toutes les licences utilisant ce secret : REJETÉES

**Transitions** :
- `INVALIDÉ → ABSENT` : Si dernier secret invalide (nettoyage)
- Pas de retour en arrière (irréversible)

**Durée** : Permanent (jusqu'à nettoyage TPM)

**Nettoyage** :
- Optionnel : Effacement du secret depuis TPM
- Recommandé : Conservation pour audit (durée limitée)

## 2. Machine à États Détaillée

### 2.1 Diagramme de Transitions

```
                    ┌─────────┐
                    │  ABSENT │
                    └────┬────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
         │               │               │
    [réception      [expiration      [corruption
     secret]         tous secrets]     irrécupérable]
         │               │               │
         ▼               ▼               ▼
    ┌─────────┐     ┌─────────┐     ┌─────────┐
    │  ACTIF  │────▶│  GRACE  │────▶│INVALIDÉ │
    └────┬────┘     └────┬────┘     └─────────┘
         │               │
    [rotation]      [expiration
     réussie]        grace_period]
         │               │
         └───────────────┘
              │
              ▼
         ┌─────────┐
         │  ACTIF  │ (nouveau)
         └─────────┘
```

### 2.2 Table de Transitions

| État Source | Événement | État Cible | Conditions | Actions |
|-------------|-----------|------------|------------|---------|
| ABSENT | Réception secret | ACTIF | Signature valide, dates OK | Stockage TPM, activation |
| ACTIF | Rotation réussie | GRACE | Nouveau secret reçu | Passage ancien en GRACE, activation nouveau |
| ACTIF | Expiration | ABSENT | `now() >= valid_until` ET pas de secret GRACE | Alerte, arrêt validations |
| ACTIF | Invalidation manuelle | INVALIDÉ | Action admin | Log audit, désactivation |
| GRACE | Expiration grace | INVALIDÉ | `now() >= grace_until` | Log audit, nettoyage optionnel |
| GRACE | Invalidation manuelle | INVALIDÉ | Action admin | Log audit, désactivation |
| * | Corruption | ABSENT | Détection corruption | Alerte, récupération |

## 3. Gestion Multi-Secrets

### 3.1 Coexistence

**Principe** : Plusieurs secrets peuvent coexister simultanément.

**Exemple** :
```
Secret v1 : ACTIF (valid_until = T+24h)
Secret v2 : GRACE (valid_until = T, grace_until = T+7j)
Secret v3 : GRACE (valid_until = T-24h, grace_until = T+6j)
```

**Règles** :
1. Un seul secret ACTIF à la fois
2. Plusieurs secrets GRACE possibles
3. Secrets INVALIDÉ conservés pour audit (optionnel)

### 3.2 Résolution de Version

**Algorithme** :

```rust
fn resolve_secret(version: u64) -> Option<Secret> {
    // 1. Vérifier secret ACTIF
    if version == active_secret.version {
        return Some(active_secret);
    }
    
    // 2. Chercher dans secrets GRACE
    for grace_secret in grace_secrets.iter() {
        if grace_secret.version == version {
            // Vérifier expiration
            if now() < grace_secret.grace_until {
                return Some(grace_secret);
            } else {
                // Expiration, invalider
                invalidate_secret(grace_secret.version);
                return None;
            }
        }
    }
    
    // 3. Secret non trouvé
    None
}
```

### 3.3 Limites de Coexistence

**Contraintes** :
- Nombre maximum de secrets GRACE : 3-5 (configurable)
- Durée totale de coexistence : `rotation_interval + grace_period`
- Nettoyage automatique des secrets expirés

**Justification** :
- Limite l'utilisation mémoire TPM
- Réduit la complexité de résolution
- Maintient la sécurité (secrets anciens supprimés)

## 4. Métadonnées et Traçabilité

### 4.1 Structure Métadonnées

```rust
struct SecretMetadata {
    version: u64,
    state: SecretState,  // ABSENT | ACTIF | GRACE | INVALIDÉ
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    grace_until: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
    rotation_source: RotationSource,  // AUTOMATIC | MANUAL | RECOVERY
    invalidation_reason: Option<String>,
}
```

### 4.2 Événements Traçables

**Événements d'état** :
- `secret_created` : Création nouveau secret
- `secret_activated` : Activation secret ACTIF
- `secret_grace_started` : Passage en GRACE
- `secret_grace_expired` : Expiration période grâce
- `secret_invalidated` : Invalidation manuelle
- `secret_corrupted` : Détection corruption

**Événements opérationnels** :
- `rotation_initiated` : Début rotation
- `rotation_succeeded` : Rotation réussie
- `rotation_failed` : Échec rotation
- `validation_attempt` : Tentative validation licence
- `validation_success` : Validation réussie
- `validation_failed` : Échec validation

### 4.3 Logs d'Audit

**Format** :
```json
{
    "timestamp": "2024-01-15T10:30:00Z",
    "event": "secret_activated",
    "secret_version": 5,
    "previous_state": "ABSENT",
    "new_state": "ACTIF",
    "valid_from": "2024-01-15T10:30:00Z",
    "valid_until": "2024-01-16T10:30:00Z",
    "agent_id": "pos-001",
    "rotation_source": "AUTOMATIC"
}
```

## 5. Gestion des Erreurs

### 5.1 Échec de Rotation

**Scénario** : Rotation échoue (réseau, serveur, signature invalide)

**Comportement** :
1. Secret ACTIF reste actif
2. Tentative de retry avec backoff exponentiel
3. Si échec prolongé : Passage en mode dégradé
4. Alerte générée après N tentatives

**Transitions** :
- `ACTIF → ACTIF` (reste actif, retry rotation)
- Si expiration approche : Alerte critique

### 5.2 Corruption de Secret

**Scénario** : Détection corruption données TPM

**Comportement** :
1. Tentative de récupération depuis backup (si disponible)
2. Si échec : Passage en ABSENT
3. Alerte critique immédiate
4. Tentative de récupération depuis serveur

**Transitions** :
- `ACTIF → ABSENT` (corruption secret actif)
- `GRACE → ABSENT` (corruption secret grace, si dernier)

### 5.3 Désynchronisation Version

**Scénario** : Version reçue ne correspond pas à l'attendu

**Comportement** :
1. Vérification avec serveur (sync)
2. Si serveur confirme : Acceptation avec alerte
3. Si serveur rejette : Rejet avec alerte

**Exemple** :
- Agent attend v5, reçoit v7
- Vérification serveur : v6 et v7 valides (v5 expiré)
- Acceptation v7, passage v6 en GRACE si nécessaire

## 6. Politiques de Nettoyage

### 6.1 Nettoyage Automatique

**Déclenchement** : Tâche périodique (ex: toutes les heures)

**Actions** :
1. Identification secrets INVALIDÉ anciens (> 30 jours)
2. Optionnel : Effacement depuis TPM
3. Conservation métadonnées pour audit

**Configuration** :
```toml
[cleanup]
enabled = true
interval_seconds = 3600
retention_days = 30
erase_invalidated = false  # Sécurité vs Audit
```

### 6.2 Nettoyage Manuel

**Via interface de gestion** :
- Invalidation immédiate d'un secret
- Nettoyage forcé des secrets expirés
- Réinitialisation complète (tous secrets)

**Sécurité** :
- Authentification forte requise
- Audit de toutes les actions
- Confirmation pour actions destructives

## 7. Récupération et Résilience

### 7.1 Récupération après Panne

**Scénarios** :
1. **Redémarrage système** : État restauré depuis TPM
2. **Perte TPM** : Récupération depuis serveur (nouvelle clé)
3. **Corruption état** : Restauration depuis backup

**Processus** :
```
1. Démarrage Agent
2. Tentative lecture état depuis TPM
3. Si succès : Restauration état
4. Si échec : Tentative backup
5. Si échec : État ABSENT, récupération serveur
```

### 7.2 Backup et Restauration

**Stratégie** :
- Backup périodique métadonnées (pas des secrets)
- Stockage chiffré
- Rotation des backups
- Test de restauration régulier

**Contenu backup** :
```json
{
    "agent_id": "pos-001",
    "active_version": 5,
    "grace_versions": [4, 3],
    "last_rotation": "2024-01-15T10:30:00Z",
    "metadata": {
        "v5": { /* métadonnées */ },
        "v4": { /* métadonnées */ },
        "v3": { /* métadonnées */ }
    }
}
```

**Note** : Les secrets eux-mêmes ne sont jamais sauvegardés (sécurité).

## 8. Métriques et Monitoring

### 8.1 Métriques d'État

- `license_agent_secret_state{state="ACTIF"}` : Nombre secrets ACTIF
- `license_agent_secret_state{state="GRACE"}` : Nombre secrets GRACE
- `license_agent_secret_state{state="INVALIDÉ"}` : Nombre secrets INVALIDÉ
- `license_agent_secret_state{state="ABSENT"}` : 1 si ABSENT, 0 sinon

### 8.2 Métriques de Transition

- `license_agent_transitions_total{from="ACTIF",to="GRACE"}` : Rotations réussies
- `license_agent_transitions_total{from="GRACE",to="INVALIDÉ"}` : Expirations grace
- `license_agent_transitions_total{from="*",to="ABSENT"}` : Erreurs critiques

### 8.3 Alertes

**Seuils** :
- État ABSENT > 5 minutes : Alerte critique
- Secret ACTIF expire dans < 1h : Alerte warning
- Plus de 3 secrets GRACE : Alerte info (nettoyage nécessaire)
- Échec rotation > 3 tentatives : Alerte warning

## 9. Conclusion

Le cycle de vie du secret garantit :

1. **Continuité** : Pas d'interruption lors des rotations
2. **Sécurité** : Secrets expirés invalidés proprement
3. **Traçabilité** : Tous les changements d'état sont loggés
4. **Résilience** : Gestion des erreurs et récupération
5. **Performance** : Résolution rapide de version

Les transitions sont claires, déterministes et sécurisées, avec une gestion complète des cas d'erreur.
