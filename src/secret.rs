use crate::tpm::TpmManager;
use crate::types::{AgentError, AgentResult, Secret, SecretMetadata, SecretState};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use zeroize::Zeroize;

/// Gestionnaire de secrets
pub struct SecretManager {
    tpm: Arc<TpmManager>,
    secrets: HashMap<u64, SecretMetadata>,
    active_version: Option<u64>,
    state_path: PathBuf,
}

impl SecretManager {
    pub fn new(tpm: Arc<TpmManager>, state_path: PathBuf) -> Self {
        Self {
            tpm,
            secrets: HashMap::new(),
            active_version: None,
            state_path,
        }
    }

    /// Charge l'état depuis le disque
    pub async fn load_state(&mut self) -> AgentResult<()> {
        if !self.state_path.exists() {
            info!("State file does not exist, starting fresh");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.state_path)
            .await
            .map_err(|e| AgentError::InternalError(format!("Failed to read state: {}", e)))?;

        let state: StateFile = serde_json::from_str(&content)
            .map_err(|e| AgentError::InternalError(format!("Failed to parse state: {}", e)))?;

        self.secrets = state.secrets;
        self.active_version = state.active_version;

        info!("State loaded: {} secrets, active version: {:?}", 
              self.secrets.len(), self.active_version);

        Ok(())
    }

    /// Sauvegarde l'état sur le disque
    pub async fn save_state(&self) -> AgentResult<()> {
        let state = StateFile {
            secrets: self.secrets.clone(),
            active_version: self.active_version,
            last_updated: Utc::now(),
        };

        let content = serde_json::to_string_pretty(&state)
            .map_err(|e| AgentError::InternalError(format!("Failed to serialize state: {}", e)))?;

        // Créer répertoire si nécessaire
        if let Some(parent) = self.state_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AgentError::InternalError(format!("Failed to create state dir: {}", e)))?;
        }

        tokio::fs::write(&self.state_path, content)
            .await
            .map_err(|e| AgentError::InternalError(format!("Failed to write state: {}", e)))?;

        debug!("State saved to {}", self.state_path.display());
        Ok(())
    }

    /// Stocke un nouveau secret
    pub async fn store_secret(&mut self, secret: Secret, version: u64) -> AgentResult<()> {
        // Chiffrer le secret avec TPM
        let encrypted = self.tpm.encrypt(&secret.data)
            .map_err(|e| AgentError::TpmError(format!("Failed to encrypt secret: {}", e)))?;

        // Stocker dans TPM NV Index (ou fallback)
        let nv_index = Self::nv_index_for_version(version);
        self.tpm.nv_write(nv_index, &encrypted)
            .map_err(|e| AgentError::TpmError(format!("Failed to store secret in TPM: {}", e)))?;

        // Mettre à jour métadonnées
        let mut metadata = secret.metadata;
        metadata.last_used_at = Some(Utc::now());
        
        self.secrets.insert(version, metadata);
        
        // Si c'est le premier secret ou si c'est une rotation, mettre à jour version active
        if self.active_version.is_none() || version > self.active_version.unwrap() {
            self.active_version = Some(version);
        }

        // Sauvegarder état
        self.save_state().await?;

        info!("Secret version {} stored successfully", version);
        Ok(())
    }

    /// Récupère un secret par version
    pub async fn get_secret(&self, version: u64) -> AgentResult<Secret> {
        // Vérifier que le secret existe
        let metadata = self.secrets.get(&version)
            .ok_or_else(|| AgentError::SecretNotFound(version))?;

        // Vérifier état
        match metadata.state {
            SecretState::Invalide => {
                return Err(AgentError::SecretInvalid(format!("Secret {} is invalidated", version)));
            }
            SecretState::Absent => {
                return Err(AgentError::SecretNotFound(version));
            }
            _ => {}
        }

        // Vérifier expiration
        let now = Utc::now();
        if now > metadata.valid_until {
            // Vérifier si en grace period
            if let Some(grace_until) = metadata.grace_until {
                if now > grace_until {
                    return Err(AgentError::SecretExpired(version));
                }
            } else {
                return Err(AgentError::SecretExpired(version));
            }
        }

        // Lire depuis TPM
        let nv_index = Self::nv_index_for_version(version);
        let encrypted = self.tpm.nv_read(nv_index)
            .map_err(|e| AgentError::TpmError(format!("Failed to read secret from TPM: {}", e)))?;

        if encrypted.is_empty() {
            return Err(AgentError::SecretNotFound(version));
        }

        // Déchiffrer
        let data = self.tpm.decrypt(&encrypted)
            .map_err(|e| AgentError::TpmError(format!("Failed to decrypt secret: {}", e)))?;

        Ok(Secret {
            data,
            metadata: metadata.clone(),
        })
    }

    /// Récupère le secret actif
    pub async fn get_active_secret(&self) -> AgentResult<Secret> {
        let version = self.active_version
            .ok_or_else(|| AgentError::SecretNotFound(0))?;
        
        self.get_secret(version).await
    }

    /// Récupère tous les secrets en état GRACE
    pub async fn get_grace_secrets(&self) -> AgentResult<Vec<Secret>> {
        let mut grace_secrets = Vec::new();

        for (version, metadata) in &self.secrets {
            if metadata.state == SecretState::Grace {
                match self.get_secret(*version).await {
                    Ok(secret) => grace_secrets.push(secret),
                    Err(e) => {
                        warn!("Failed to load grace secret {}: {}", version, e);
                    }
                }
            }
        }

        Ok(grace_secrets)
    }

    /// Passe un secret en état GRACE
    pub async fn set_grace(&mut self, version: u64, grace_until: DateTime<Utc>) -> AgentResult<()> {
        let metadata = self.secrets.get_mut(&version)
            .ok_or_else(|| AgentError::SecretNotFound(version))?;

        metadata.state = SecretState::Grace;
        metadata.grace_until = Some(grace_until);

        self.save_state().await?;
        info!("Secret {} set to GRACE until {}", version, grace_until);

        Ok(())
    }

    /// Invalide un secret
    pub async fn invalidate(&mut self, version: u64, reason: Option<String>) -> AgentResult<()> {
        let metadata = self.secrets.get_mut(&version)
            .ok_or_else(|| AgentError::SecretNotFound(version))?;

        metadata.state = SecretState::Invalide;
        metadata.invalidation_reason = reason;

        // Si c'était le secret actif, chercher un nouveau secret actif
        if self.active_version == Some(version) {
            self.active_version = self.find_new_active_version();
        }

        self.save_state().await?;
        info!("Secret {} invalidated", version);

        Ok(())
    }

    /// Nettoie les secrets expirés
    pub async fn cleanup_expired(&mut self) -> AgentResult<usize> {
        let now = Utc::now();
        let mut cleaned = 0;

        let expired_versions: Vec<u64> = self.secrets
            .iter()
            .filter_map(|(version, metadata)| {
                match metadata.state {
                    SecretState::Grace => {
                        if let Some(grace_until) = metadata.grace_until {
                            if now > grace_until {
                                Some(*version)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        for version in expired_versions {
            if let Err(e) = self.invalidate(version, Some("Grace period expired".to_string())).await {
                error!("Failed to invalidate expired secret {}: {}", version, e);
            } else {
                cleaned += 1;
            }
        }

        if cleaned > 0 {
            info!("Cleaned up {} expired secrets", cleaned);
        }

        Ok(cleaned)
    }

    fn find_new_active_version(&self) -> Option<u64> {
        self.secrets
            .iter()
            .filter(|(_, metadata)| metadata.state == SecretState::Actif)
            .map(|(version, _)| *version)
            .max()
    }

    fn nv_index_for_version(version: u64) -> u32 {
        // NV Index de base + version
        // Note: TPM 2.0 a des contraintes sur les NV Index, ajuster selon besoin
        0x01000000 + (version as u32 & 0x00FFFFFF)
    }

    /// Obtient les métadonnées d'un secret (sans le secret lui-même)
    pub fn get_metadata(&self, version: u64) -> Option<&SecretMetadata> {
        self.secrets.get(&version)
    }

    /// Obtient la version active
    pub fn active_version(&self) -> Option<u64> {
        self.active_version
    }

    /// Liste toutes les versions de secrets
    pub fn list_versions(&self) -> Vec<u64> {
        self.secrets.keys().copied().collect()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StateFile {
    secrets: HashMap<u64, SecretMetadata>,
    active_version: Option<u64>,
    last_updated: DateTime<Utc>,
}
