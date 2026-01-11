# Stratégie de Rotation sans Coupure

## 1. Principe Fondamental

**Objectif** : Renouveler le secret de déchiffrement sans interrompre la validation des licences.

**Contrainte critique** : Les licences en cours d'utilisation peuvent référencer différentes versions de secret. La rotation ne doit pas invalider les licences valides.

**Solution** : Coexistence temporaire de plusieurs secrets avec transition progressive.

## 2. Fenêtre de Rotation

### 2.1 Déclenchement

**Critères de déclenchement** :

```
1. Temporel (principal)
   if now() >= (active_secret.valid_until - rotation_threshold):
       initiate_rotation()
   
   rotation_threshold = min(rotation_interval * 0.1, 1h)
   Exemple : Si rotation_interval = 24h, threshold = 1h
             Rotation déclenchée à T+23h

2. Manuel
   - Via interface de gestion
   - Action admin authentifiée

3. Serveur push (optionnel)
   - Notification serveur pour rotation urgente
   - Vérification cryptographique obligatoire
```

**Justification** :
- Déclenchement proactif avant expiration
- Fenêtre suffisante pour gérer les échecs
- Pas de déclenchement trop tôt (sécurité)

### 2.2 Timing Optimal

**Calcul de la fenêtre** :

```
rotation_window = active_secret.valid_until - now()

Conditions :
- rotation_window >= rotation_threshold : Rotation normale
- rotation_threshold > rotation_window > 0 : Rotation urgente
- rotation_window <= 0 : Mode dégradé (secret expiré)
```

**Stratégie** :
- **Fenêtre large (> 1h)** : Rotation normale, retry possible
- **Fenêtre étroite (< 1h)** : Rotation urgente, alerte
- **Fenêtre nulle** : Secret expiré, mode dégradé activé

## 3. Processus de Rotation

### 3.1 Phase 1 : Préparation

**Actions** :
```
1. Vérification état actuel
   - Secret ACTIF identifié
   - Version confirmée
   - Dates validées

2. Préparation requête
   - Génération nonce unique
   - Horodatage
   - Signature avec clé agent

3. Vérification connectivité
   - Test connexion serveur
   - Vérification certificat
   - Si échec : Mode dégradé

4. Lock rotation (éviter doublons)
   - Mutex global
   - Timeout 5 minutes
```

**Durée** : < 100ms (synchrones)

### 3.2 Phase 2 : Communication Serveur

**Actions** :
```
1. Envoi requête rotation
   POST /api/v1/rotate-secret
   {
       agent_id,
       current_version,
       timestamp,
       nonce,
       signature
   }

2. Attente réponse
   - Timeout : 30 secondes
   - Retry : 3 tentatives avec backoff
   - Backoff : 1s, 2s, 4s

3. Vérification réponse
   - Signature serveur
   - Version cohérente
   - Dates valides
   - Nonce correspondant
```

**Durée** : 100ms - 30s (selon réseau)

**Gestion erreurs** :
- **Timeout** : Retry avec backoff
- **Signature invalide** : Alerte, pas de retry
- **Version incohérente** : Sync avec serveur
- **Échec total** : Mode dégradé

### 3.3 Phase 3 : Réception et Validation

**Actions** :
```
1. Réception nouveau secret
   - Déchiffrement
   - Vérification intégrité

2. Validation métadonnées
   - version == current_version + 1
   - valid_from <= now() + tolerance
   - valid_until > now()
   - grace_until > valid_until

3. Préparation stockage
   - Chiffrement avec TPM
   - Préparation NV Index
```

**Durée** : 50-200ms (opérations TPM)

**Validation stricte** :
- Rejet si version incorrecte
- Rejet si dates incohérentes
- Rejet si signature invalide

### 3.4 Phase 4 : Coexistence (Transition)

**Actions** :
```
1. Stockage nouveau secret
   - Écriture TPM (nouveau NV Index)
   - Métadonnées : état ACTIF
   - Version : new_version

2. Passage ancien secret en GRACE
   - Mise à jour métadonnées
   - État : GRACE
   - grace_until : new_secret.grace_until (ou calculé)

3. Activation nouveau secret
   - Mise à jour active_version
   - Nouveau secret devient ACTIF
   - Ancien secret devient GRACE

4. Vérification cohérence
   - Les deux secrets accessibles
   - Résolution version fonctionnelle
```

**Durée** : 100-300ms (opérations TPM)

**Atomicité** :
- Transaction TPM si possible
- Rollback en cas d'échec partiel
- État cohérent garanti

### 3.5 Phase 5 : Finalisation

**Actions** :
```
1. Confirmation serveur (optionnel)
   POST /api/v1/rotation-confirmed
   {
       agent_id,
       new_version,
       timestamp
   }

2. Log audit
   - Événement rotation_succeeded
   - Ancienne version
   - Nouvelle version
   - Durée rotation

3. Nettoyage
   - Libération mutex
   - Mise à jour métriques
   - Notification monitoring
```

**Durée** : 50-100ms

**Optionnel** :
- Confirmation serveur non bloquante
- Si échec : Log uniquement, pas d'impact

## 4. Gestion de la Coexistence

### 4.1 Période de Coexistence

**Durée** :
```
coexistence_duration = grace_period
Exemple : 7 jours
```

**Pendant cette période** :
- Secret ACTIF : Utilisé pour nouvelles licences
- Secret GRACE : Utilisé pour licences existantes (version correspondante)

### 4.2 Résolution de Version

**Algorithme** :

```rust
fn get_secret_for_license(license_token: &[u8]) -> Result<Secret> {
    // 1. Extraction version depuis token
    let license_version = extract_version(license_token);
    
    // 2. Vérification secret ACTIF
    if license_version == active_secret.version {
        return Ok(active_secret);
    }
    
    // 3. Recherche dans secrets GRACE
    for grace_secret in grace_secrets.iter() {
        if grace_secret.version == license_version {
            // Vérification expiration
            if now() < grace_secret.grace_until {
                return Ok(grace_secret);
            } else {
                // Expiration, invalider
                invalidate_secret(grace_secret.version);
                return Err(SecretExpired);
            }
        }
    }
    
    // 4. Secret non trouvé
    Err(SecretNotFound)
}
```

**Performance** :
- Recherche O(1) pour secret ACTIF
- Recherche O(n) pour secrets GRACE (n limité à 3-5)
- Cache des résultats fréquents

### 4.3 Limites de Coexistence

**Contraintes** :
- **Nombre maximum** : 3-5 secrets GRACE (configurable)
- **Durée totale** : `rotation_interval + grace_period`
- **Nettoyage** : Automatique après expiration

**Justification** :
- Limite mémoire TPM
- Réduit complexité résolution
- Maintient sécurité (suppression anciens secrets)

## 5. Gestion des Échecs

### 5.1 Échec Communication

**Scénario** : Impossible de contacter le serveur

**Comportement** :
```
1. Retry avec backoff exponentiel
   - Tentative 1 : Immédiate
   - Tentative 2 : +1s
   - Tentative 3 : +2s
   - Tentative 4 : +4s
   - Maximum : 3-5 tentatives

2. Si échec total :
   - Secret ACTIF reste actif
   - Mode dégradé activé
   - Alerte générée
   - Retry périodique (toutes les 5 minutes)
```

**Impact** :
- Aucune interruption service
- Secret ACTIF continue de fonctionner
- Rotation reportée

### 5.2 Échec Validation

**Scénario** : Réponse serveur invalide (signature, version, dates)

**Comportement** :
```
1. Rejet immédiat (pas de retry)
   - Signature invalide : Alerte sécurité
   - Version incohérente : Sync avec serveur
   - Dates invalides : Alerte configuration

2. Secret ACTIF reste actif
3. Log audit détaillé
4. Notification admin
```

**Impact** :
- Aucune interruption service
- Rotation reportée
- Investigation requise

### 5.3 Échec Stockage TPM

**Scénario** : Impossible d'écrire dans TPM

**Comportement** :
```
1. Tentative de récupération
   - Vérification espace TPM
   - Nettoyage anciens secrets si nécessaire
   - Retry écriture

2. Si échec persistant :
   - Rollback (annulation rotation)
   - Secret ACTIF reste actif
   - Alerte critique
   - Fallback chiffrement disque (si configuré)
```

**Impact** :
- Rotation annulée
- Service continue avec secret ACTIF
- Intervention manuelle requise

### 5.4 Échec Partiel (État Incohérent)

**Scénario** : Rotation partiellement réussie (ex: nouveau secret stocké mais ancien pas en GRACE)

**Comportement** :
```
1. Détection incohérence
   - Vérification état TPM
   - Comparaison avec état attendu

2. Récupération automatique
   - Complétion opération manquante
   - Ou rollback complet

3. Si récupération impossible :
   - État ABSENT
   - Alerte critique
   - Récupération depuis serveur
```

**Impact** :
- Service peut être interrompu temporairement
- Récupération automatique si possible
- Sinon intervention manuelle

## 6. Stratégie de Retry

### 6.1 Retry Communication

**Politique** :
```
max_retries = 3
base_delay = 1s
max_delay = 30s
backoff = exponential

Tentative 1 : Immédiate
Tentative 2 : +1s
Tentative 3 : +2s
Tentative 4 : +4s
...
```

**Conditions d'arrêt** :
- Succès
- Max retries atteint
- Erreur non récupérable (signature invalide)

### 6.2 Retry Périodique

**Si rotation échoue** :
```
1. Mode dégradé activé
2. Retry périodique toutes les 5 minutes
3. Jusqu'à succès ou expiration secret ACTIF
4. Alerte si expiration approche (< 1h)
```

**Justification** :
- Réseau peut être temporairement indisponible
- Retry périodique permet récupération automatique
- Alerte prévient expiration critique

## 7. Validation de la Rotation

### 7.1 Vérifications Post-Rotation

**Immédiatement après rotation** :
```
1. Nouveau secret accessible
   - Lecture TPM réussie
   - Déchiffrement fonctionnel

2. Ancien secret en GRACE
   - Métadonnées correctes
   - Accessible pour résolution

3. État cohérent
   - Un seul secret ACTIF
   - Versions séquentielles
   - Dates cohérentes

4. Test validation
   - Test avec licence nouvelle version
   - Test avec licence ancienne version
```

**Durée** : < 500ms

### 7.2 Monitoring Continu

**Métriques** :
- `rotation_duration_seconds` : Durée totale rotation
- `rotation_success_total` : Rotations réussies
- `rotation_failed_total` : Échecs rotation
- `secrets_active` : Nombre secrets ACTIF (devrait être 1)
- `secrets_grace` : Nombre secrets GRACE

**Alertes** :
- Rotation > 1 minute : Alerte performance
- Échec rotation : Alerte warning
- Plusieurs secrets ACTIF : Alerte critique (incohérence)

## 8. Optimisations

### 8.1 Pré-chargement

**Stratégie** :
- Charger handles TPM au démarrage
- Cache des clés fréquemment utilisées
- Préparation NV Index avant rotation

**Bénéfice** : Réduction latence rotation (50-100ms)

### 8.2 Rotation Asynchrone

**Option** : Rotation en arrière-plan

**Avantages** :
- Pas de blocage validations
- Meilleure expérience utilisateur

**Inconvénients** :
- Complexité accrue
- Gestion erreurs plus complexe

**Recommandation** : Rotation synchrone (simplicité, sécurité)

### 8.3 Batch Rotations

**Si plusieurs agents** :
- Coordination serveur pour éviter pic charge
- Distribution temporelle des rotations

**Côté agent** : Pas d'impact (rotation indépendante)

## 9. Cas Limites

### 9.1 Rotation Simultanée

**Scénario** : Deux rotations déclenchées simultanément

**Protection** :
- Mutex global
- Timeout 5 minutes
- Une seule rotation à la fois

**Comportement** :
- Première rotation : Exécution normale
- Deuxième rotation : Attente ou rejet (déjà en cours)

### 9.2 Rotation pendant Validation

**Scénario** : Rotation déclenchée pendant validation licence

**Comportement** :
- Validation continue avec secret actuel
- Rotation en parallèle (si asynchrone)
- Ou rotation après validation (si synchrone)

**Recommandation** : Rotation non bloquante pour validations

### 9.3 Secret ACTIF Expiré

**Scénario** : Rotation échoue, secret ACTIF expire

**Comportement** :
```
1. Secret ACTIF expire
2. Passage en GRACE (si possible)
3. Ou passage en ABSENT
4. Mode dégradé activé
5. Alerte critique
6. Retry rotation continue
```

**Impact** :
- Nouvelles licences peuvent être rejetées
- Licences existantes (version GRACE) continuent
- Récupération dès rotation réussie

## 10. Conclusion

La stratégie de rotation sans coupure garantit :

1. **Continuité** : Aucune interruption service
2. **Sécurité** : Validation cryptographique stricte
3. **Résilience** : Gestion complète des échecs
4. **Performance** : Rotation rapide (< 1s dans cas normal)
5. **Traçabilité** : Logs complets de toutes les opérations

La coexistence temporaire des secrets permet une transition en douceur, avec une fenêtre de grâce suffisante pour gérer les cas d'usage réels.
