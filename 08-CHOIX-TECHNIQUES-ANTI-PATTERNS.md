# Choix Techniques et Anti-Patterns

## 1. Justification des Choix Techniques

### 1.1 Langage : Rust

**Choix** : Rust comme langage principal pour le Secret Agent.

**Justifications** :

1. **Sécurité mémoire** :
   - Pas de buffer overflow
   - Pas de use-after-free
   - Pas de double free
   - Garanties compile-time

2. **Performance** :
   - Performance native (pas de VM)
   - Pas de garbage collector (latence prévisible)
   - Optimisations compilateur avancées

3. **Écosystème cryptographique** :
   - `ring` : Cryptographie moderne (TLS, signatures)
   - `tpm2-rs` : Bindings TPM 2.0
   - `zeroize` : Zeroization mémoire sécurisée

4. **Sécurité par défaut** :
   - Pas de comportements undefined
   - Vérifications à la compilation
   - Pas de null pointer exceptions

5. **Concurrence sûre** :
   - Ownership system empêche data races
   - Pas de race conditions mémoire

**Alternatives considérées** :
- **C/C++** : Performance mais risques sécurité mémoire
- **Go** : Simplicité mais GC peut causer latence
- **Python** : Trop lent et risques sécurité

**Conclusion** : Rust est le meilleur compromis sécurité/performance pour ce cas d'usage.

### 1.2 TPM 2.0

**Choix** : Utilisation de TPM 2.0 pour stockage sécurisé.

**Justifications** :

1. **Protection matérielle** :
   - Secret stocké dans matériel sécurisé
   - Non exportable (clé avec flag approprié)
   - Résistant aux attaques logicielles

2. **Isolation** :
   - Pas d'accès direct depuis logiciel
   - Opérations cryptographiques dans TPM
   - Protection contre extraction mémoire

3. **Standards** :
   - Standard industriel (TCG)
   - Support large (Intel, AMD, etc.)
   - Bibliothèques matures

4. **Fonctionnalités** :
   - NV Index pour stockage persistant
   - PCR pour mesure d'intégrité
   - Attestation possible

**Alternatives considérées** :
- **HSM externe** : Plus cher, nécessite matériel supplémentaire
- **Chiffrement disque uniquement** : Moins sécurisé (extraction possible)
- **Secure Enclave (Intel SGX/AMD SEV)** : Plus complexe, support variable

**Conclusion** : TPM 2.0 offre le meilleur rapport sécurité/coût/disponibilité.

### 1.3 Unix Domain Socket

**Choix** : Communication POS Application ↔ Secret Agent via Unix Domain Socket.

**Justifications** :

1. **Sécurité** :
   - Pas d'exposition réseau
   - Contrôle UID via `SO_PEERCRED`
   - Permissions fichiers système

2. **Performance** :
   - Pas de sérialisation réseau
   - Latence minimale
   - Pas de overhead TCP/IP

3. **Simplicité** :
   - Pas de configuration réseau
   - Pas de firewall
   - Débogage facile

**Alternatives considérées** :
- **TCP localhost** : Exposition réseau (même si local)
- **Named pipes (FIFO)** : Moins flexible
- **Shared memory** : Plus complexe, risques sécurité

**Conclusion** : Unix Domain Socket est optimal pour communication locale sécurisée.

### 1.4 Service Systemd

**Choix** : Déploiement comme service systemd.

**Justifications** :

1. **Intégration système** :
   - Démarrage automatique
   - Gestion de cycle de vie
   - Logs système intégrés

2. **Sécurité** :
   - Isolation via systemd (PrivateTmp, ProtectSystem)
   - Contrôle capabilities
   - Restart automatique

3. **Standard** :
   - Présent sur toutes distributions modernes
   - Configuration standardisée
   - Outils de gestion intégrés

**Alternatives considérées** :
- **Init scripts** : Obsolète, moins sécurisé
- **Supervisor** : Moins intégré au système
- **Docker** : Overhead, complexité inutile

**Conclusion** : systemd est le standard moderne et sécurisé.

### 1.5 AES-256-GCM pour Licences

**Choix** : AES-256-GCM pour chiffrement des licences.

**Justifications** :

1. **Sécurité** :
   - AES-256 : Standard NIST, résistant aux attaques quantiques (pour l'instant)
   - GCM : Authenticated encryption (intégrité + confidentialité)
   - IV unique : Protection contre replay

2. **Performance** :
   - Accélération matérielle (AES-NI)
   - Efficace pour petits messages
   - Pas de padding (GCM)

3. **Standard** :
   - Recommandé par NIST, ANSSI
   - Support large
   - Implémentations auditées

**Alternatives considérées** :
- **AES-CBC** : Moins sécurisé (pas d'authentification intégrée)
- **ChaCha20-Poly1305** : Bon mais moins standard
- **RSA uniquement** : Trop lent pour données volumineuses

**Conclusion** : AES-256-GCM est le choix optimal sécurité/performance.

### 1.6 RSA-OAEP pour Transmission Secret

**Choix** : RSA-OAEP pour chiffrement du secret lors de la transmission.

**Justifications** :

1. **Sécurité** :
   - OAEP : Padding sécurisé (contre attaques)
   - RSA-2048 minimum (RSA-4096 recommandé)
   - Protection contre attaques adaptatives

2. **Asymétrie** :
   - Permet chiffrement avec clé publique
   - Seul détenteur clé privée peut déchiffrer
   - Pas de partage secret symétrique

3. **Standard** :
   - Recommandé par NIST, RFC 8017
   - Support large
   - Implémentations auditées

**Alternatives considérées** :
- **RSA-PKCS1v1.5** : Moins sécurisé (padding vulnérable)
- **ECC (ECDH + AES)** : Plus moderne mais complexité ajoutée
- **Secret partagé** : Nécessite canal sécurisé préalable

**Conclusion** : RSA-OAEP est le standard éprouvé pour ce cas d'usage.

### 1.7 Coexistence Multi-Secrets

**Choix** : Support de plusieurs secrets simultanés (ACTIF + GRACE).

**Justifications** :

1. **Continuité** :
   - Pas d'interruption lors rotation
   - Licences existantes continuent de fonctionner
   - Transition en douceur

2. **Sécurité** :
   - Fenêtre de grâce limitée
   - Secrets anciens invalidés automatiquement
   - Pas d'accumulation infinie

3. **Pragmatisme** :
   - Réalité des déploiements (licences en transit)
   - Gestion des cas d'usage réels
   - Évite problèmes opérationnels

**Alternatives considérées** :
- **Rotation instantanée** : Interruption service possible
- **Pas de rotation** : Sécurité diminuée (secret statique)
- **Rotation avec downtime** : Inacceptable pour POS

**Conclusion** : Coexistence multi-secrets est nécessaire pour continuité.

### 1.8 Mode Dégradé avec Grace Period

**Choix** : Mode dégradé avec période de grâce limitée.

**Justifications** :

1. **Résilience** :
   - Fonctionnement temporaire sans réseau
   - Gestion pannes réseau
   - Continuité service

2. **Sécurité** :
   - Limitation durée (grace period)
   - Pas d'abus possible
   - Traçabilité complète

3. **Réalité opérationnelle** :
   - Réseaux POS parfois instables
   - Maintenance planifiée
   - Cas d'usage réels

**Alternatives considérées** :
- **Pas de mode dégradé** : Interruption service en cas panne réseau
- **Mode dégradé illimité** : Risque sécurité (contournement contrôle)
- **Mode dégradé avec validation serveur** : Contradictoire (pas de réseau)

**Conclusion** : Mode dégradé limité est le bon compromis résilience/sécurité.

## 2. Anti-Patterns à Éviter Absolument

### 2.1 ❌ Stockage Secret en Clair

**Anti-pattern** :
```rust
// MAUVAIS
let secret = "my-secret-key-12345";
fs::write("/etc/license-agent/secret.txt", secret)?;
```

**Problèmes** :
- Secret lisible par root
- Apparaît dans backups
- Extraction facile
- Violation principe fondamental

**Solution** :
- Stockage TPM (chiffré)
- Jamais en clair sur disque
- Zeroization après usage

### 2.2 ❌ Transmission Secret via Logs

**Anti-pattern** :
```rust
// MAUVAIS
log::info!("Secret reçu: {}", secret);
println!("Debug: secret = {:?}", secret);
```

**Problèmes** :
- Secret dans logs système
- Accessible via journalctl
- Apparaît dans dumps
- Traçabilité compromission

**Solution** :
- Jamais logger le secret
- Logger uniquement métadonnées (version, dates)
- Vérification automatique (tests)

### 2.3 ❌ Secret dans Variables d'Environnement

**Anti-pattern** :
```bash
# MAUVAIS
export LICENSE_SECRET="my-secret"
./license-agent
```

**Problèmes** :
- Visible via `ps aux`
- Dans `/proc/*/environ`
- Accessible à tous processus enfants
- Pas de protection

**Solution** :
- Secret depuis TPM uniquement
- Pas de variables d'environnement
- Pas de fichiers de configuration en clair

### 2.4 ❌ Rotation sans Validation Cryptographique

**Anti-pattern** :
```rust
// MAUVAIS
let new_secret = receive_from_server();
store_secret(new_secret); // Pas de vérification signature
```

**Problèmes** :
- Injection secrets malveillants possible
- Pas d'authentification serveur
- Attaques man-in-the-middle
- Compromission totale

**Solution** :
- Vérification signature obligatoire
- Validation certificat serveur
- Vérification nonce (anti-replay)
- Validation dates et versions

### 2.5 ❌ Secret Partagé via IPC Non Sécurisé

**Anti-pattern** :
```rust
// MAUVAIS
let secret = get_secret();
send_to_application(secret); // Via socket non authentifié
```

**Problèmes** :
- Secret exposé à tous processus
- Pas de contrôle accès
- Interception possible
- Violation isolation

**Solution** :
- Unix Domain Socket avec SO_PEERCRED
- Vérification UID/GID
- Whitelist processus autorisés
- Secret jamais transmis (validation uniquement)

### 2.6 ❌ Pas de Zeroization Mémoire

**Anti-pattern** :
```rust
// MAUVAIS
let secret = decrypt_secret();
use_secret(secret);
// secret reste en mémoire indéfiniment
```

**Problèmes** :
- Secret récupérable via dump mémoire
- Reste en mémoire après usage
- Accessible via debugger
- Compromission post-mortem

**Solution** :
- Zeroization automatique (Drop trait)
- `zeroize` crate
- Mlock pour pages mémoire
- Pas de core dumps

### 2.7 ❌ Rotation avec Downtime

**Anti-pattern** :
```rust
// MAUVAIS
deactivate_old_secret();
wait_for_all_validations_to_finish();
activate_new_secret();
```

**Problèmes** :
- Interruption service
- Validations rejetées pendant transition
- Expérience utilisateur dégradée
- Perte revenus (POS)

**Solution** :
- Coexistence temporaire
- Transition progressive
- Pas de downtime
- Rotation transparente

### 2.8 ❌ Mode Dégradé Illimité

**Anti-pattern** :
```rust
// MAUVAIS
if network_unavailable {
    degraded_mode = true;
    // Pas de limite de durée
}
```

**Problèmes** :
- Contournement contrôle serveur
- Pas de supervision
- Risque sécurité
- Abus possible

**Solution** :
- Grace period limitée
- Alertes progressives
- Expiration automatique
- Traçabilité complète

### 2.9 ❌ Pas de Vérification Intégrité

**Anti-pattern** :
```rust
// MAUVAIS
let license = decrypt_license(token);
return license; // Pas de vérification signature
```

**Problèmes** :
- Modification licence possible
- Injection données malveillantes
- Pas d'authenticité garantie
- Compromission fonctionnelle

**Solution** :
- Vérification signature (GCM auth tag)
- Validation métadonnées
- Vérification dates expiration
- Validation règles métier

### 2.10 ❌ Secret Statique (Pas de Rotation)

**Anti-pattern** :
```rust
// MAUVAIS
const SECRET: &[u8] = b"static-secret-forever";
// Jamais changé
```

**Problèmes** :
- Si compromis, compromis pour toujours
- Pas de limitation dégâts
- Pas de récupération
- Risque sécurité croissant

**Solution** :
- Rotation régulière (24h)
- Secrets à durée de vie limitée
- Invalidation automatique
- Récupération possible

### 2.11 ❌ Pas d'Audit

**Anti-pattern** :
```rust
// MAUVAIS
invalidate_secret(version);
// Pas de log
```

**Problèmes** :
- Pas de traçabilité
- Pas de détection anomalies
- Pas de conformité
- Pas de débogage

**Solution** :
- Logs d'audit complets
- Toutes actions tracées
- Horodatage cryptographique (optionnel)
- Export pour analyse

### 2.12 ❌ Gestion Erreurs Insuffisante

**Anti-pattern** :
```rust
// MAUVAIS
let secret = get_secret().unwrap(); // Panic si échec
```

**Problèmes** :
- Crash en cas d'erreur
- Pas de récupération
- Perte disponibilité
- Expérience utilisateur dégradée

**Solution** :
- Gestion erreurs complète
- Retry avec backoff
- Mode dégradé si nécessaire
- Récupération automatique

### 2.13 ❌ Pas de Rate Limiting

**Anti-pattern** :
```rust
// MAUVAIS
// Interface de gestion sans limite
```

**Problèmes** :
- Attaques par déni de service
- Surcharge système
- Extraction par brute force
- Pas de protection

**Solution** :
- Rate limiting par admin
- Token bucket algorithm
- Blocage temporaire
- Alertes si abus

### 2.14 ❌ Configuration Non Sécurisée

**Anti-pattern** :
```toml
# MAUVAIS
[server]
secret = "my-secret-in-config"  # En clair dans config
```

**Problèmes** :
- Secret dans fichiers
- Accessible via backups
- Versioning (git) expose secret
- Pas de protection

**Solution** :
- Pas de secret dans config
- Secrets depuis TPM uniquement
- Config signée (intégrité)
- Permissions strictes (600)

### 2.15 ❌ Pas de Vérification Version

**Anti-pattern** :
```rust
// MAUVAIS
let new_secret = receive_secret();
store_secret(new_secret); // Pas de vérification version
```

**Problèmes** :
- Désynchronisation possible
- Replay d'anciens secrets
- État incohérent
- Sécurité compromise

**Solution** :
- Vérification version séquentielle
- Rejet si version incorrecte
- Sync avec serveur si nécessaire
- Validation stricte

## 3. Bonnes Pratiques à Suivre

### 3.1 ✅ Défense en Profondeur

- Plusieurs couches de sécurité
- Pas de point de défaillance unique
- Vérifications multiples

### 3.2 ✅ Principe du Moindre Privilège

- Permissions minimales
- Isolation processus
- Pas d'accès inutile

### 3.3 ✅ Fail-Safe

- En cas d'erreur : Rejet plutôt qu'acceptation
- Mode dégradé plutôt que compromission
- Alerte plutôt que silence

### 3.4 ✅ Traçabilité Complète

- Toutes actions loggées
- Horodatage précis
- Contexte complet

### 3.5 ✅ Validation Cryptographique

- Signatures vérifiées
- Certificats validés
- Nonces pour anti-replay

### 3.6 ✅ Zeroization

- Secret effacé après usage
- Mémoire nettoyée
- Pas de résidus

### 3.7 ✅ Tests de Sécurité

- Tests d'injection
- Tests de résilience
- Tests de performance
- Tests de récupération

## 4. Conclusion

Les choix techniques sont justifiés par :
- **Sécurité** : Protection maximale du secret
- **Performance** : Latence minimale
- **Résilience** : Gestion erreurs complète
- **Maintenabilité** : Code clair et testé

Les anti-patterns identifiés doivent être évités absolument pour garantir la sécurité du système. La mise en œuvre doit suivre les bonnes pratiques énoncées.
