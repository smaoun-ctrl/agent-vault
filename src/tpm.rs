use crate::types::AgentError;
use anyhow::Result;
use std::sync::Arc;
use tss_esapi::{Context, TctiNameConf};
use tracing::{debug, error, info, warn};

/// Gestionnaire TPM
pub struct TpmManager {
    context: Option<Arc<Context>>,
    enabled: bool,
}

impl TpmManager {
    pub fn new(enabled: bool) -> Result<Self> {
        let context = if enabled {
            match Self::create_context() {
                Ok(ctx) => {
                    info!("TPM context created successfully");
                    Some(Arc::new(ctx))
                }
                Err(e) => {
                    warn!("Failed to create TPM context: {}. Falling back to software encryption.", e);
                    None
                }
            }
        } else {
            info!("TPM disabled, using software encryption fallback");
            None
        };

        Ok(Self { context, enabled })
    }

    fn create_context() -> Result<Context> {
        let tcti = TctiNameConf::from_environment_variable()
            .unwrap_or_else(|_| TctiNameConf::Mssim {
                host: "localhost".to_string(),
                port: 2321,
            });

        let mut context = Context::new(tcti)?;
        context.initialize()?;
        Ok(context)
    }

    pub fn is_available(&self) -> bool {
        self.context.is_some()
    }

    /// Chiffre des données avec TPM
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, AgentError> {
        if let Some(ctx) = &self.context {
            // Utiliser une clé TPM pour chiffrer
            // Note: Implémentation simplifiée, nécessite configuration clé TPM
            self.encrypt_with_tpm(ctx, data)
        } else {
            // Fallback: chiffrement logiciel (moins sécurisé)
            warn!("Using software encryption fallback (TPM not available)");
            self.encrypt_software(data)
        }
    }

    /// Déchiffre des données avec TPM
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, AgentError> {
        if let Some(ctx) = &self.context {
            self.decrypt_with_tpm(ctx, encrypted)
        } else {
            self.decrypt_software(encrypted)
        }
    }

    fn encrypt_with_tpm(&self, ctx: &Context, data: &[u8]) -> Result<Vec<u8>, AgentError> {
        // TODO: Implémenter chiffrement avec clé TPM persistante
        // Pour l'instant, fallback logiciel
        self.encrypt_software(data)
    }

    fn decrypt_with_tpm(&self, ctx: &Context, encrypted: &[u8]) -> Result<Vec<u8>, AgentError> {
        // TODO: Implémenter déchiffrement avec clé TPM persistante
        self.decrypt_software(encrypted)
    }

    fn encrypt_software(&self, data: &[u8]) -> Result<Vec<u8>, AgentError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use rand::Rng;

        // Génération clé depuis variable d'environnement ou fichier
        // NOTE: Ceci est un fallback temporaire, doit être remplacé par TPM
        let key = self.get_fallback_key()?;
        let cipher = Aes256Gcm::new(&key);
        
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| AgentError::CryptoError(format!("Encryption failed: {}", e)))?;

        // Préfixer avec nonce
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    fn decrypt_software(&self, encrypted: &[u8]) -> Result<Vec<u8>, AgentError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };

        if encrypted.len() < 12 {
            return Err(AgentError::CryptoError("Invalid encrypted data".to_string()));
        }

        let key = self.get_fallback_key()?;
        let cipher = Aes256Gcm::new(&key);

        let nonce = Nonce::from_slice(&encrypted[0..12]);
        let ciphertext = &encrypted[12..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AgentError::CryptoError(format!("Decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    fn get_fallback_key(&self) -> Result<aes_gcm::Key<Aes256Gcm>, AgentError> {
        use aes_gcm::KeyInit;
        use sha2::{Digest, Sha256};

        // Dérivation clé depuis fichier ou variable d'environnement
        // WARNING: Ceci est un fallback temporaire
        let seed = std::env::var("LICENSE_AGENT_FALLBACK_KEY")
            .unwrap_or_else(|_| "CHANGE_THIS_IN_PRODUCTION".to_string());
        
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let key_bytes = hasher.finalize();
        
        Ok(aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes).clone())
    }

    /// Écrit dans un NV Index TPM
    pub fn nv_write(&self, index: u32, data: &[u8]) -> Result<(), AgentError> {
        if let Some(_ctx) = &self.context {
            // TODO: Implémenter écriture NV Index complète avec tss-esapi
            // let nv_index = NvIndexTpmHandle::new(index)?;
            // let auth = NvAuth::Password;
            // ctx.nv_write(nv_index, auth, data)?;
            debug!("NV write to index {} ({} bytes)", index, data.len());
            Ok(())
        } else {
            Err(AgentError::TpmError("TPM not available".to_string()))
        }
    }

    /// Lit depuis un NV Index TPM
    pub fn nv_read(&self, index: u32) -> Result<Vec<u8>, AgentError> {
        if let Some(_ctx) = &self.context {
            // TODO: Implémenter lecture NV Index complète avec tss-esapi
            // let nv_index = NvIndexTpmHandle::new(index)?;
            // let auth = NvAuth::Password;
            // let data = ctx.nv_read(nv_index, auth, size)?;
            debug!("NV read from index {}", index);
            Ok(vec![])
        } else {
            Err(AgentError::TpmError("TPM not available".to_string()))
        }
    }

    /// Obtient le statut TPM
    pub fn get_status(&self) -> crate::types::TpmStatus {
        use crate::types::TpmStatus;
        
        if let Some(ctx) = &self.context {
            // TODO: Récupérer informations TPM réelles
            TpmStatus {
                available: true,
                version: Some("2.0".to_string()),
                manufacturer: None,
                firmware_version: None,
                keys_loaded: 0,
                nv_space_used: None,
            }
        } else {
            TpmStatus {
                available: false,
                version: None,
                manufacturer: None,
                firmware_version: None,
                keys_loaded: 0,
                nv_space_used: None,
            }
        }
    }
}

impl Drop for TpmManager {
    fn drop(&mut self) {
        if let Some(ctx) = Arc::try_unwrap(self.context.take().unwrap_or_else(|| {
            // Créer un contexte temporaire pour le drop
            Self::create_context().ok().map(Arc::new).unwrap()
        })) {
            if let Err(e) = ctx.teardown() {
                error!("Failed to teardown TPM context: {}", e);
            }
        }
    }
}
