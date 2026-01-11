# Flux Cryptographiques Détaillés

## 1. Génération et Distribution Initiale

### 1.1 Génération des Clés Serveur

**Côté serveur de licences** :

```
1. Génération paire de clés RSA-4096 (ou ECC P-384)
   - Clé privée serveur : K_priv_server
   - Clé publique serveur : K_pub_server

2. Stockage sécurisé de K_priv_server
   - HSM ou TPM serveur
   - Chiffrement au repos
   - Accès restreint

3. Distribution de K_pub_server
   - Intégrée dans le binaire POS
   - Ou fichier de configuration signé
   - Vérification intégrité à chaque démarrage
```

### 1.2 Génération du Secret Symétrique

**Côté serveur de licences** :

```
1. Génération secret symétrique
   secret = random_bytes(32)  // AES-256
   version = 1
   valid_from = now()
   valid_until = now() + rotation_interval
   grace_until = valid_until + grace_period

2. Chiffrement du secret avec clé publique agent
   secret_encrypted = RSA_OAEP_encrypt(
       K_pub_agent,
       secret,
       label = "license-secret-v1"
   )

3. Signature du paquet
   signature = RSA_PSS_sign(
       K_priv_server,
       hash = SHA-256(
           secret_encrypted ||
           version ||
           valid_from ||
           valid_until ||
           grace_until ||
           agent_id
       )
   )

4. Envoi au POS
   {
       secret_encrypted: bytes,
       version: u64,
       valid_from: timestamp,
       valid_until: timestamp,
       grace_until: timestamp,
       signature: bytes
   }
```

### 1.3 Réception et Stockage sur POS

**Côté Secret Agent** :

```
1. Réception du paquet depuis serveur

2. Vérification signature
   hash = SHA-256(
       secret_encrypted ||
       version ||
       valid_from ||
       valid_until ||
       grace_until ||
       agent_id
   )
   if !RSA_PSS_verify(K_pub_server, hash, signature):
       REJECT

3. Déchiffrement du secret
   secret = RSA_OAEP_decrypt(
       K_priv_agent,  // Stockée dans TPM
       secret_encrypted
   )

4. Stockage dans TPM
   // Chiffrement avec clé TPM
   tpm_key_handle = TPM2_Load(
       parent_handle,
       K_priv_agent_encrypted
   )
   
   secret_encrypted_tpm = TPM2_EncryptDecrypt(
       tpm_key_handle,
       secret,
       mode = ENCRYPT
   )
   
   // Stockage dans NV Index
   TPM2_NV_Write(
       nv_index_secret,
       secret_encrypted_tpm,
       auth = TPM policy
   )
   
   // Métadonnées
   TPM2_NV_Write(
       nv_index_metadata,
       {
           version: u64,
           valid_from: timestamp,
           valid_until: timestamp,
           grace_until: timestamp,
           state: ACTIF
       }
   )

5. Zeroization mémoire
   secure_zero_memory(secret)
```

## 2. Chiffrement et Validation de Licence

### 2.1 Génération de Licence (Serveur)

**Côté serveur de licences** :

```
1. Création payload licence
   payload = {
       license_id: uuid,
       customer_id: string,
       features: [string],
       expires_at: timestamp,
       metadata: {...}
   }

2. Sérialisation
   payload_json = JSON.serialize(payload)
   payload_bytes = payload_json.as_bytes()

3. Chiffrement avec secret symétrique
   // Génération IV unique
   iv = random_bytes(12)  // AES-GCM
   
   // Chiffrement
   (ciphertext, auth_tag) = AES_256_GCM_encrypt(
       key = secret,
       iv = iv,
       plaintext = payload_bytes,
       aad = license_id  // Additional Authenticated Data
   )

4. Construction token
   token = base64_encode(
       version ||      // Version du secret utilisé
       iv ||
       ciphertext ||
       auth_tag
   )

5. Envoi au POS
   license_token = token
```

### 2.2 Validation de Licence (POS)

**Côté Secret Agent** :

```
1. Réception token depuis POS Application
   token = validate_license_request.license_token

2. Décodage token
   data = base64_decode(token)
   version = extract_version(data)
   iv = extract_iv(data)
   ciphertext = extract_ciphertext(data)
   auth_tag = extract_auth_tag(data)

3. Récupération secret depuis TPM
   // Lecture métadonnées
   metadata = TPM2_NV_Read(nv_index_metadata)
   
   // Vérification version
   if metadata.version != version:
       // Chercher dans secrets en grace
       secret = find_secret_in_grace(version)
   else:
       secret = get_active_secret()
   
   // Déchiffrement depuis TPM
   secret_plain = TPM2_EncryptDecrypt(
       tpm_key_handle,
       secret_encrypted_tpm,
       mode = DECRYPT
   )

4. Déchiffrement licence
   payload_bytes = AES_256_GCM_decrypt(
       key = secret_plain,
       iv = iv,
       ciphertext = ciphertext,
       auth_tag = auth_tag,
       aad = license_id
   )
   
   if decryption_fails:
       REJECT

5. Désérialisation
   payload = JSON.deserialize(payload_bytes)

6. Validation métier
   if payload.expires_at < now():
       REJECT
   
   // Vérifications supplémentaires
   validate_license_rules(payload)

7. Zeroization
   secure_zero_memory(secret_plain)

8. Réponse à POS Application
   {
       valid: true,
       expires_at: payload.expires_at,
       features: payload.features,
       metadata: payload.metadata
   }
```

## 3. Rotation du Secret

### 3.1 Initiation de la Rotation

**Côté Secret Agent** :

```
1. Détection besoin rotation
   current_secret = get_active_secret()
   if current_secret.valid_until - now() < rotation_threshold:
       initiate_rotation()

2. Préparation requête
   agent_id = config.agent_id
   current_version = current_secret.version
   timestamp = now()
   nonce = random_bytes(16)

3. Signature requête
   request_hash = SHA-256(
       agent_id ||
       current_version ||
       timestamp ||
       nonce
   )
   
   request_signature = TPM2_Sign(
       tpm_key_handle_agent,
       request_hash,
       scheme = RSASSA_PSS
   )

4. Envoi au serveur
   POST /api/v1/rotate-secret
   {
       agent_id: string,
       current_version: u64,
       timestamp: timestamp,
       nonce: bytes,
       signature: bytes
   }
```

### 3.2 Traitement Rotation (Serveur)

**Côté serveur de licences** :

```
1. Réception requête

2. Vérification signature
   request_hash = SHA-256(
       agent_id ||
       current_version ||
       timestamp ||
       nonce
   )
   
   // Récupération clé publique agent depuis DB
   K_pub_agent = get_agent_public_key(agent_id)
   
   if !RSA_verify(K_pub_agent, request_hash, signature):
       REJECT

3. Vérification nonce (anti-replay)
   if nonce_already_used(nonce):
       REJECT
   mark_nonce_used(nonce, ttl = 1h)

4. Vérification version
   stored_version = get_agent_secret_version(agent_id)
   if current_version != stored_version:
       // Gérer cas de désynchronisation
       handle_version_mismatch()

5. Génération nouveau secret
   new_secret = random_bytes(32)
   new_version = current_version + 1
   valid_from = now()
   valid_until = now() + rotation_interval
   grace_until = valid_until + grace_period

6. Chiffrement nouveau secret
   new_secret_encrypted = RSA_OAEP_encrypt(
       K_pub_agent,
       new_secret,
       label = "license-secret-v{new_version}"
   )

7. Signature réponse
   response_hash = SHA-256(
       new_secret_encrypted ||
       new_version ||
       valid_from ||
       valid_until ||
       grace_until ||
       agent_id ||
       nonce  // Inclure nonce pour lier requête/réponse
   )
   
   response_signature = RSA_PSS_sign(
       K_priv_server,
       response_hash
   )

8. Envoi réponse
   {
       new_secret_encrypted: bytes,
       version: new_version,
       valid_from: timestamp,
       valid_until: timestamp,
       grace_until: timestamp,
       signature: bytes
   }
```

### 3.3 Réception et Activation (POS)

**Côté Secret Agent** :

```
1. Réception réponse

2. Vérification signature
   response_hash = SHA-256(
       new_secret_encrypted ||
       version ||
       valid_from ||
       valid_until ||
       grace_until ||
       agent_id ||
       nonce  // Vérifier correspondance avec requête
   )
   
   if !RSA_PSS_verify(K_pub_server, response_hash, signature):
       REJECT

3. Vérification version
   if version != current_version + 1:
       REJECT

4. Vérification dates
   if valid_from > now() + tolerance:
       REJECT  // Secret pas encore valide
   
   if valid_until < now():
       REJECT  // Secret déjà expiré

5. Déchiffrement nouveau secret
   new_secret = RSA_OAEP_decrypt(
       K_priv_agent,
       new_secret_encrypted
   )

6. Stockage nouveau secret dans TPM
   // Même processus que stockage initial
   new_secret_encrypted_tpm = TPM2_EncryptDecrypt(
       tpm_key_handle,
       new_secret,
       mode = ENCRYPT
   )
   
   // Stockage dans nouveau NV Index ou index versionné
   TPM2_NV_Write(
       nv_index_secret_v{version},
       new_secret_encrypted_tpm
   )
   
   // Métadonnées
   TPM2_NV_Write(
       nv_index_metadata_v{version},
       {
           version: version,
           valid_from: valid_from,
           valid_until: valid_until,
           grace_until: grace_until,
           state: ACTIF
       }
   )

7. Mise à jour état ancien secret
   // Passage en GRACE
   old_metadata = TPM2_NV_Read(nv_index_metadata_v{current_version})
   old_metadata.state = GRACE
   TPM2_NV_Write(
       nv_index_metadata_v{current_version},
       old_metadata
   )

8. Activation nouveau secret
   active_secret_version = version
   active_secret_valid_from = valid_from

9. Zeroization
   secure_zero_memory(new_secret)

10. Log rotation
    audit_log("rotation_success", {
        old_version: current_version,
        new_version: version,
        timestamp: now()
    })
```

## 4. Gestion Multi-Secrets (Coexistence)

### 4.1 Structure de Stockage

```
TPM NV Indexes:
- nv_index_secret_v1: secret chiffré version 1
- nv_index_metadata_v1: métadonnées version 1
- nv_index_secret_v2: secret chiffré version 2
- nv_index_metadata_v2: métadonnées version 2
- ...
- nv_index_active_version: version actuellement active
```

### 4.2 Recherche de Secret

**Algorithme de résolution** :

```
function get_secret_for_version(requested_version):
    // 1. Vérifier secret actif
    active_version = TPM2_NV_Read(nv_index_active_version)
    if requested_version == active_version:
        metadata = TPM2_NV_Read(nv_index_metadata_v{active_version})
        if metadata.state == ACTIF:
            return get_secret_from_tpm(active_version)
    
    // 2. Chercher dans secrets en grace
    for version in [active_version - 1, active_version - 2, ...]:
        if version < 1:
            break
        
        metadata = TPM2_NV_Read(nv_index_metadata_v{version})
        if metadata.state == GRACE:
            if now() < metadata.grace_until:
                return get_secret_from_tpm(version)
            else:
                // Secret expiré, invalider
                metadata.state = INVALIDÉ
                TPM2_NV_Write(nv_index_metadata_v{version}, metadata)
    
    // 3. Secret non trouvé
    return None
```

### 4.3 Nettoyage des Secrets Expirés

**Tâche périodique** :

```
function cleanup_expired_secrets():
    active_version = TPM2_NV_Read(nv_index_active_version)
    
    for version in [1, 2, ..., active_version - max_grace_versions]:
        metadata = TPM2_NV_Read(nv_index_metadata_v{version})
        
        if metadata.state == GRACE:
            if now() > metadata.grace_until:
                // Expiration grace period
                metadata.state = INVALIDÉ
                TPM2_NV_Write(nv_index_metadata_v{version}, metadata)
                
                // Optionnel : effacement secret (sécurité renforcée)
                // TPM2_NV_UndefineSpace(nv_index_secret_v{version})
                
                audit_log("secret_expired", {
                    version: version,
                    timestamp: now()
                })
```

## 5. Authentification Interface de Gestion

### 5.1 Authentification par Certificat

**Génération certificat client** :

```
1. Génération paire de clés
   openssl genrsa -out client.key 2048
   openssl req -new -key client.key -out client.csr

2. Signature par CA interne
   openssl x509 -req -in client.csr \
       -CA ca.crt -CAkey ca.key \
       -out client.crt -days 365

3. Stockage sécurisé
   - client.crt: permissions 644
   - client.key: permissions 600
```

**Vérification côté Agent** :

```
1. Réception requête avec certificat

2. Vérification certificat
   cert = extract_certificate(request)
   
   // Vérification chaîne
   if !verify_certificate_chain(cert, ca_cert):
       REJECT
   
   // Vérification expiration
   if cert.not_after < now():
       REJECT
   
   // Vérification révocation (CRL ou OCSP)
   if cert_revoked(cert):
       REJECT

3. Extraction identité
   subject = cert.subject
   cn = extract_cn(subject)

4. Vérification permissions
   if !is_authorized(cn, requested_action):
       REJECT

5. Signature requête
   request_hash = SHA-256(request_body)
   if !RSA_verify(cert.public_key, request_hash, request_signature):
       REJECT

6. Autorisation action
   execute_action(requested_action)
```

### 5.2 Authentification par Token

**Alternative plus simple** :

```
1. Génération token côté serveur
   token = HMAC_SHA256(
       key = master_key,
       message = agent_id || user_id || expiration
   )

2. Stockage token signé
   signed_token = base64_encode(
       user_id ||
       expiration ||
       token
   )

3. Vérification côté Agent
   (user_id, expiration, token) = base64_decode(signed_token)
   
   if expiration < now():
       REJECT
   
   expected_token = HMAC_SHA256(
       master_key,
       agent_id || user_id || expiration
   )
   
   if !constant_time_compare(token, expected_token):
       REJECT
```

## 6. Protection Mémoire

### 6.1 Zeroization

**Implémentation Rust** :

```rust
use zeroize::Zeroize;

struct Secret {
    data: Vec<u8>,
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

// Utilisation
{
    let secret = Secret { data: secret_bytes };
    // Utilisation...
} // Zeroization automatique à la sortie du scope
```

### 6.2 Protection contre Dump Mémoire

**Techniques** :

1. **Mlock** : Verrouillage pages en mémoire
   ```rust
   use libc::{mlock, munlock};
   
   unsafe {
       mlock(secret.as_ptr(), secret.len());
   }
   ```

2. **No-dump flag** : Empêcher core dumps
   ```rust
   use libc::{prctl, PR_SET_DUMPABLE};
   
   unsafe {
       prctl(PR_SET_DUMPABLE, 0);
   }
   ```

3. **Scrambling** : Mélange mémoire périodique
   ```rust
   // Réorganisation périodique des données en mémoire
   ```

## 7. Résumé des Algorithmes

### 7.1 Chiffrement Asymétrique

- **Algorithme** : RSA-OAEP (RSA-2048 minimum, RSA-4096 recommandé)
- **Hash** : SHA-256 ou SHA-512
- **Usage** : Chiffrement secret pour transmission

### 7.2 Chiffrement Symétrique

- **Algorithme** : AES-256-GCM
- **IV** : 12 bytes (96 bits) aléatoire unique
- **Usage** : Chiffrement licences

### 7.3 Signatures

- **Algorithme** : RSA-PSS (RSA-2048 minimum)
- **Hash** : SHA-256
- **Usage** : Authentification messages serveur/agent

### 7.4 Stockage TPM

- **Clé TPM** : RSA-2048 ou ECC P-256 (non exportable)
- **Chiffrement** : TPM2_EncryptDecrypt
- **Stockage** : NV Index avec politique d'authentification

### 7.5 Hash

- **Algorithme** : SHA-256 (ou SHA-512 pour sécurité renforcée)
- **Usage** : Intégrité, signatures, dérivation

## 8. Sécurité des Communications

### 8.1 TLS 1.3

**Configuration minimale** :

```
- Version : TLS 1.3 uniquement
- Cipher suites : 
  * TLS_AES_256_GCM_SHA384
  * TLS_CHACHA20_POLY1305_SHA256
- Certificats : 
  * Vérification stricte
  * Pinning certificat serveur
- Perfect Forward Secrecy : OBLIGATOIRE
- Renégociation : DÉSACTIVÉE
```

### 8.2 Authentification Mutuelle

**Certificats client/serveur** :

```
Client (Agent) :
- Certificat signé par CA interne
- Vérification côté serveur

Serveur :
- Certificat signé par CA publique ou interne
- Vérification + pinning côté client
```

## 9. Conclusion

Ces flux cryptographiques garantissent :

1. **Confidentialité** : Secret jamais en clair
2. **Intégrité** : Vérification signatures systématique
3. **Authenticité** : Authentification mutuelle
4. **Non-répudiation** : Traçabilité complète
5. **Anti-replay** : Nonces et horodatage
6. **Robustesse** : Gestion multi-versions et mode dégradé

Tous les algorithmes utilisés sont standards, éprouvés et recommandés par les autorités de sécurité (NIST, ANSSI).
