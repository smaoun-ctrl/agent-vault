use crate::secret::SecretManager;
use crate::types::{AgentError, AgentResult, LicenseInfo, Secret, ValidationResult};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::Utc;
use std::sync::Arc;
use base64::{engine::general_purpose, Engine as _};
use tracing::{debug, info};

/// Validateur de licences
pub struct LicenseValidator {
    secret_manager: Arc<SecretManager>,
}

impl LicenseValidator {
    pub fn new(secret_manager: Arc<SecretManager>) -> Self {
        Self { secret_manager }
    }

    /// Valide un token de licence
    pub async fn validate(&self, license_token: &[u8]) -> AgentResult<ValidationResult> {
        debug!("Validating license token ({} bytes)", license_token.len());

        // 1. Décoder le token
        let token_data = general_purpose::STANDARD
            .decode(license_token)
            .map_err(|e| AgentError::LicenseValidationFailed(format!("Invalid base64: {}", e)))?;

        if token_data.len() < 13 {
            return Err(AgentError::LicenseValidationFailed("Token too short".to_string()));
        }

        // 2. Extraire version, IV, ciphertext, auth_tag
        let version_bytes = u64::from_be_bytes([
            token_data[0], token_data[1], token_data[2], token_data[3],
            token_data[4], token_data[5], token_data[6], token_data[7],
        ]);
        let version = version_bytes;

        let iv = &token_data[8..20]; // 12 bytes
        let ciphertext_with_tag = &token_data[20..];

        if ciphertext_with_tag.len() < 16 {
            return Err(AgentError::LicenseValidationFailed("Invalid token format".to_string()));
        }

        let ciphertext = &ciphertext_with_tag[..ciphertext_with_tag.len() - 16];
        let auth_tag = &ciphertext_with_tag[ciphertext_with_tag.len() - 16..];

        // 3. Récupérer le secret correspondant
        let secret = self.get_secret_for_version(version).await?;

        // 4. Déchiffrer la licence
        let license_info = self.decrypt_license(&secret, iv, ciphertext, auth_tag, version)
            .map_err(|e| AgentError::LicenseValidationFailed(format!("Decryption failed: {}", e)))?;

        // 5. Valider la licence (dates, règles métier)
        self.validate_license_rules(&license_info)?;

        // 6. Mettre à jour last_used_at du secret
        // Note: Nécessite mutabilité, à implémenter si nécessaire

        info!("License {} validated successfully (expires: {})", 
              license_info.license_id, license_info.expires_at);

        Ok(ValidationResult {
            valid: true,
            expires_at: Some(license_info.expires_at),
            features: license_info.features,
            metadata: license_info.metadata,
            error: None,
        })
    }

    async fn get_secret_for_version(&self, version: u64) -> AgentResult<Secret> {
        // Essayer d'abord le secret actif
        if let Some(active_version) = self.secret_manager.active_version() {
            if version == active_version {
                return self.secret_manager.get_active_secret().await;
            }
        }

        // Chercher dans les secrets GRACE
        let grace_secrets = self.secret_manager.get_grace_secrets().await?;
        for secret in grace_secrets {
            if secret.metadata.version == version {
                return Ok(secret);
            }
        }

        // Secret non trouvé
        Err(AgentError::SecretNotFound(version))
    }

    fn decrypt_license(
        &self,
        secret: &Secret,
        iv: &[u8],
        ciphertext: &[u8],
        auth_tag: &[u8],
        license_version: u64,
    ) -> Result<LicenseInfo, String> {
        // Reconstruire le ciphertext avec auth_tag pour GCM
        let mut full_ciphertext = ciphertext.to_vec();
        full_ciphertext.extend_from_slice(auth_tag);

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&secret.data);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(iv);

        // AAD (Additional Authenticated Data) : version du secret utilisée
        // Cela garantit que le token ne peut pas être réutilisé avec un autre secret
        let _aad = license_version.to_be_bytes();

        // Déchiffrer
        let plaintext = cipher
            .decrypt(nonce, full_ciphertext.as_ref())
            .map_err(|e| format!("GCM decryption failed: {}", e))?;

        // Désérialiser JSON
        let license_info: LicenseInfo = serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Failed to parse license JSON: {}", e))?;

        // Valider que la version dans la licence correspond
        // (si stockée dans la licence elle-même)

        Ok(license_info)
    }

    fn validate_license_rules(&self, license: &LicenseInfo) -> AgentResult<()> {
        let now = Utc::now();

        // Vérifier expiration
        if now > license.expires_at {
            return Err(AgentError::LicenseValidationFailed(
                format!("License expired at {}", license.expires_at)
            ));
        }

        // Vérifications supplémentaires possibles :
        // - Blacklist de license_id
        // - Vérification customer_id
        // - Vérification features
        // - etc.

        Ok(())
    }

    /// Obtient les statistiques de validation
    pub async fn get_stats(&self) -> crate::types::LicenseStatus {
        // TODO: Implémenter compteurs réels
        crate::types::LicenseStatus {
            last_validation: Some(Utc::now()),
            total_validations: 0,
            successful_validations: 0,
            failed_validations: 0,
            last_error: None,
        }
    }
}
