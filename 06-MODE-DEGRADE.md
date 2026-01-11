# Gestion du Mode Dégradé

## 1. Définition et Objectifs

### 1.1 Qu'est-ce que le Mode Dégradé ?

Le mode dégradé est un état opérationnel où le système continue de fonctionner avec les secrets déjà valides, mais sans possibilité de recevoir de nouveaux secrets depuis le serveur distant.

**Caractéristiques** :
- Validation des licences existantes : **AUTORISÉE**
- Réception de nouveaux secrets : **IMPOSSIBLE**
- Génération de nouvelles licences : **IMPOSSIBLE** (côté serveur)
- Durée limitée : **Grace period** (ex: 7 jours)

### 1.2 Objectifs

1. **Continuité de service** : Permettre le fonctionnement temporaire sans connexion Internet
2. **Sécurité** : Limiter la durée d'opération sans supervision serveur
3. **Traçabilité** : Logger toutes les opérations en mode dégradé
4. **Récupération** : Retour automatique au mode normal dès reconnexion

## 2. Conditions d'Activation

### 2.1 Activation Automatique

**Scénarios déclencheurs** :

```
1. Indisponibilité réseau
   - Échec connexion serveur
   - Timeout répétés
   - Pas de réponse serveur

2. Échec rotation prolongé
   - Rotation échoue > 3 tentatives
   - Pas de nouveau secret reçu
   - Secret ACTIF approche expiration

3. Secret ACTIF expiré
   - valid_until < now()
   - Pas de nouveau secret disponible
   - Secrets GRACE encore valides

4. Serveur inaccessible
   - DNS resolution échoue
   - Certificat invalide (non récupérable)
   - Serveur retourne erreur 5xx
```

**Critères de déclenchement** :
```
if (connexion_serveur_impossible && 
    secret_actif_valide && 
    !mode_degrade_actif):
    activate_degraded_mode()
```

### 2.2 Activation Manuelle

**Via interface de gestion** :
- Action admin authentifiée
- Justification requise (optionnel)
- Audit obligatoire

**Cas d'usage** :
- Maintenance réseau planifiée
- Test de résilience
- Dépannage

**Sécurité** :
- Authentification forte requise
- Rôle admin uniquement
- Log audit détaillé
- Confirmation action

## 3. Comportement en Mode Dégradé

### 3.1 Validations de Licence

**Règles** :

```
1. Licences avec secret ACTIF
   - Si secret ACTIF valide : ACCEPTÉES
   - Si secret ACTIF expiré : REJETÉES

2. Licences avec secret GRACE
   - Si secret GRACE valide (grace_until > now()) : ACCEPTÉES
   - Si secret GRACE expiré : REJETÉES

3. Licences avec version inconnue
   - REJETÉES (pas de nouveau secret possible)
```

**Algorithme** :

```rust
fn validate_license_degraded(license_token: &[u8]) -> Result<LicenseInfo> {
    let version = extract_version(license_token);
    
    // 1. Vérifier secret ACTIF
    if version == active_secret.version {
        if now() < active_secret.valid_until {
            return validate_with_secret(active_secret, license_token);
        } else {
            // Secret ACTIF expiré
            return Err(LicenseRejected::SecretExpired);
        }
    }
    
    // 2. Vérifier secrets GRACE
    for grace_secret in grace_secrets.iter() {
        if grace_secret.version == version {
            if now() < grace_secret.grace_until {
                return validate_with_secret(grace_secret, license_token);
            } else {
                // Secret GRACE expiré
                invalidate_secret(grace_secret.version);
                return Err(LicenseRejected::SecretExpired);
            }
        }
    }
    
    // 3. Version inconnue
    Err(LicenseRejected::SecretNotFound)
}
```

### 3.2 Tentatives de Rotation

**Comportement** :

```
1. Rotation automatique désactivée
   - Pas de déclenchement automatique
   - Tentatives manuelles possibles (si réseau disponible)

2. Retry périodique
   - Toutes les 5 minutes
   - Test connexion serveur
   - Si succès : Tentative rotation
   - Si rotation réussie : Désactivation mode dégradé

3. Limite de retry
   - Pas de limite temporelle
   - Limite : Expiration tous secrets
```

**Configuration** :
```toml
[degraded_mode]
retry_interval_seconds = 300  # 5 minutes
max_retry_duration = null     # Pas de limite (jusqu'à expiration)
```

### 3.3 Rejet de Nouveaux Secrets

**Règle stricte** : Aucun nouveau secret ne doit être accepté en mode dégradé, sauf si la connexion serveur est vérifiée.

**Justification** :
- Éviter injection de secrets malveillants
- Garantir authenticité via serveur
- Protection contre attaques man-in-the-middle

**Exception** : Si connexion serveur réussit et signature valide, accepter nouveau secret et désactiver mode dégradé.

## 4. Grace Period

### 4.1 Définition

**Grace period** : Durée maximale pendant laquelle le mode dégradé peut rester actif.

**Valeur typique** : 7 jours (configurable)

**Calcul** :
```
grace_period_end = min(
    active_secret.grace_until,
    now() + grace_period_duration
)
```

### 4.2 Expiration de la Grace Period

**Comportement** :

```
1. Détection expiration
   if now() >= grace_period_end:
       handle_grace_period_expired()

2. Actions
   - Passage état ABSENT (si dernier secret)
   - Arrêt validations nouvelles licences
   - Licences existantes (si secret GRACE valide) : Continuer
   - Alerte critique immédiate
   - Notification admin

3. Récupération
   - Reconnexion serveur requise
   - Nouveau secret obligatoire
   - Réinitialisation si nécessaire
```

### 4.3 Avertissements Progressifs

**Seuils d'alerte** :

```
- T+0h (activation) : Alerte info
- T+24h : Alerte warning
- T+72h (3 jours) : Alerte warning renforcée
- T+144h (6 jours) : Alerte critical
- T+168h (7 jours) : Alerte critical + expiration
```

**Actions** :
- Logs d'audit à chaque seuil
- Métriques exportées
- Notifications (si configuré)

## 5. Métriques et Monitoring

### 5.1 Métriques Mode Dégradé

```
license_agent_degraded_mode_active{value="1"} : Mode dégradé actif
license_agent_degraded_mode_duration_seconds : Durée depuis activation
license_agent_degraded_mode_grace_period_remaining_seconds : Temps restant
license_agent_degraded_mode_validations_total : Validations en mode dégradé
license_agent_degraded_mode_validations_failed : Échecs validation
license_agent_degraded_mode_rotation_attempts_total : Tentatives rotation
license_agent_degraded_mode_rotation_success_total : Rotations réussies
```

### 5.2 Alertes

**Conditions** :

| Condition | Niveau | Action |
|-----------|--------|--------|
| Activation mode dégradé | Info | Log audit |
| Mode dégradé > 24h | Warning | Notification |
| Mode dégradé > 72h | Warning | Notification renforcée |
| Mode dégradé > 144h | Critical | Alerte admin |
| Expiration grace period | Critical | Arrêt service partiel |
| Rotation réussie | Info | Désactivation mode dégradé |

### 5.3 Dashboard

**Informations affichées** :
- État mode dégradé (actif/inactif)
- Durée depuis activation
- Temps restant avant expiration
- Nombre validations en mode dégradé
- Dernière tentative rotation
- Raison activation

## 6. Désactivation du Mode Dégradé

### 6.1 Désactivation Automatique

**Conditions** :

```
1. Rotation réussie
   - Nouveau secret reçu et validé
   - Secret ACTIF mis à jour
   - Connexion serveur confirmée
   
   → Désactivation automatique

2. Reconnexion serveur
   - Test connexion réussit
   - Vérification certificat OK
   - Pas de rotation nécessaire (secret ACTIF valide)
   
   → Désactivation automatique (optionnel, selon config)
```

**Processus** :

```rust
fn deactivate_degraded_mode() {
    // 1. Vérification prérequis
    if !server_connection_available() {
        return; // Pas de désactivation si serveur inaccessible
    }
    
    // 2. Vérification secret ACTIF
    if active_secret.valid_until < now() + threshold {
        // Secret expire bientôt, rotation requise
        if !rotation_successful() {
            return; // Pas de désactivation si rotation échoue
        }
    }
    
    // 3. Désactivation
    degraded_mode_active = false;
    degraded_mode_activated_at = None;
    
    // 4. Log audit
    audit_log("degraded_mode_deactivated", {
        duration: now() - degraded_mode_activated_at,
        reason: "automatic_recovery"
    });
    
    // 5. Notification
    notify_degraded_mode_deactivated();
}
```

### 6.2 Désactivation Manuelle

**Via interface de gestion** :
- Action admin authentifiée
- Vérification connexion serveur
- Confirmation requise

**Cas d'usage** :
- Test de récupération
- Dépannage
- Maintenance

**Sécurité** :
- Authentification forte
- Audit obligatoire
- Vérification serveur (optionnel mais recommandé)

## 7. Traçabilité

### 7.1 Logs d'Audit

**Événements tracés** :

```json
// Activation
{
    "timestamp": "2024-01-15T10:30:00Z",
    "event": "degraded_mode_activated",
    "reason": "network_unavailable",
    "active_secret_version": 5,
    "active_secret_valid_until": "2024-01-16T10:30:00Z",
    "grace_period_end": "2024-01-22T10:30:00Z"
}

// Validations
{
    "timestamp": "2024-01-15T11:00:00Z",
    "event": "license_validated_degraded",
    "license_id": "uuid",
    "secret_version": 5,
    "result": "accepted"
}

// Tentatives rotation
{
    "timestamp": "2024-01-15T11:05:00Z",
    "event": "rotation_attempt_degraded",
    "result": "failed",
    "error": "network_timeout"
}

// Désactivation
{
    "timestamp": "2024-01-15T12:00:00Z",
    "event": "degraded_mode_deactivated",
    "duration_seconds": 5400,
    "reason": "rotation_successful",
    "new_secret_version": 6
}
```

### 7.2 Rapport Mode Dégradé

**Génération** : À la désactivation ou sur demande

**Contenu** :
- Durée totale mode dégradé
- Raison activation
- Nombre validations
- Nombre échecs
- Tentatives rotation
- Événements significatifs

## 8. Cas Limites

### 8.1 Activation pendant Rotation

**Scénario** : Mode dégradé activé pendant rotation en cours

**Comportement** :
- Rotation continue (si déjà initiée)
- Si rotation réussit : Mode dégradé désactivé
- Si rotation échoue : Mode dégradé confirmé

### 8.2 Secret ACTIF Expiré en Mode Dégradé

**Scénario** : Secret ACTIF expire pendant mode dégradé

**Comportement** :
```
1. Secret ACTIF expire
2. Passage en GRACE (si possible)
3. Si secrets GRACE disponibles : Continuer validations
4. Si aucun secret valide : État ABSENT
5. Alerte critique
```

### 8.3 Tous Secrets Expirés

**Scénario** : Tous secrets (ACTIF + GRACE) expirent en mode dégradé

**Comportement** :
```
1. Détection expiration
2. Passage état ABSENT
3. Toutes validations rejetées
4. Alerte critique immédiate
5. Récupération serveur obligatoire
```

## 9. Configuration

### 9.1 Paramètres

```toml
[degraded_mode]
enabled = true                    # Activer mode dégradé
grace_period_days = 7             # Durée grace period
retry_interval_seconds = 300     # Intervalle retry rotation
auto_deactivate_on_reconnect = true  # Désactivation auto si reconnexion
alert_thresholds = [24h, 72h, 144h]  # Seuils d'alerte
```

### 9.2 Politiques

**Politique stricte** :
- Grace period courte (3 jours)
- Alertes fréquentes
- Désactivation auto uniquement après rotation

**Politique permissive** :
- Grace period longue (14 jours)
- Alertes moins fréquentes
- Désactivation auto sur reconnexion

**Recommandation** : Politique équilibrée (7 jours, alertes progressives)

## 10. Sécurité

### 10.1 Protection contre Abus

**Risque** : Utilisation prolongée mode dégradé pour contourner contrôle serveur

**Mitigation** :
- Grace period limitée (non configurable côté client)
- Alertes progressives
- Traçabilité complète
- Détection anomalies (trop de validations)

### 10.2 Validation Cryptographique

**Règle** : Même en mode dégradé, toutes les validations utilisent la cryptographie complète.

**Garanties** :
- Signature licence vérifiée
- Secret TPM utilisé
- Pas de bypass sécurité

### 10.3 Isolation

**Règle** : Mode dégradé n'affecte que la réception nouveaux secrets, pas la sécurité des validations.

## 11. Conclusion

Le mode dégradé garantit :

1. **Continuité** : Fonctionnement temporaire sans connexion
2. **Sécurité** : Limitation durée et validation cryptographique
3. **Traçabilité** : Logs complets de toutes opérations
4. **Récupération** : Retour automatique au mode normal
5. **Alertes** : Notifications progressives avant expiration

Le système reste sécurisé et opérationnel même en cas d'indisponibilité réseau temporaire, avec des garde-fous stricts pour éviter les abus.
