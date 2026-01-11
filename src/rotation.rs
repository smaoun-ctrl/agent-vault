use crate::audit::AuditLogger;
use crate::config::Config;
use crate::crypto::CryptoManager;
use crate::secret::SecretManager;
use crate::types::{AgentError, AgentResult, RotationSource, Secret, SecretMetadata, SecretState};
use chrono::{DateTime, Utc};
use rand::Rng;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use zeroize::Zeroize;

/// Gestionnaire de rotation
pub struct RotationManager {
    config: Arc<Config>,
    secret_manager: Arc<SecretManager>,
    audit: Arc<AuditLogger>,
    client: Client,
    crypto: Arc<CryptoManager>,
    rotation_in_progress: Arc<tokio::sync::Mutex<bool>>,
    max_retries: u32,
    base_retry_delay_seconds: u64,
}

#[derive(Debug, serde::Serialize)]
struct RotateSecretRequest {
    agent_id: String,
    current_version: u64,
    timestamp: DateTime<Utc>,
    nonce: String,
    signature: String, // TODO: Implémenter signature réelle
}

#[derive(Debug, serde::Deserialize)]
struct RotateSecretResponse {
    new_secret_encrypted: String, // Base64
    version: u64,
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    grace_until: DateTime<Utc>,
    signature: String,
}

impl RotationManager {
    pub fn new(
        config: Arc<Config>,
        secret_manager: Arc<SecretManager>,
        audit: Arc<AuditLogger>,
        crypto: Arc<CryptoManager>,
    ) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.server.timeout_seconds.unwrap_or(30)))
            .build()?;

        Ok(Self {
            config,
            secret_manager,
            audit,
            client,
            crypto,
            rotation_in_progress: Arc::new(tokio::sync::Mutex::new(false)),
            max_retries: 3,
            base_retry_delay_seconds: 1,
        })
    }

    /// Configure le nombre max de retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Configure le délai de base pour retry
    pub fn with_base_retry_delay(mut self, seconds: u64) -> Self {
        self.base_retry_delay_seconds = seconds;
        self
    }

    /// Vérifie si une rotation est nécessaire
    pub async fn check_rotation_needed(&self) -> bool {
        if let Some(active_version) = self.secret_manager.active_version() {
            if let Some(metadata) = self.secret_manager.get_metadata(active_version) {
                let now = Utc::now();
                let threshold = Duration::from_secs(
                    self.config.agent.rotation_threshold_seconds.unwrap_or(3600), // 1h par défaut
                );
                
                if metadata.valid_until.signed_duration_since(now).num_seconds() 
                    < threshold.as_secs() as i64 {
                    return true;
                }
            }
        } else {
            // Pas de secret actif, rotation nécessaire
            return true;
        }

        false
    }

    /// Déclenche une rotation
    pub async fn rotate(&self, force: bool) -> AgentResult<()> {
        // Vérifier si rotation déjà en cours
        {
            let mut in_progress = self.rotation_in_progress.lock().await;
            if *in_progress {
                return Err(AgentError::RotationFailed("Rotation already in progress".to_string()));
            }
            *in_progress = true;
        }

        let start_time = std::time::Instant::now();

        // Libérer le lock en cas d'erreur
        let result = self.do_rotate(force).await;

        {
            let mut in_progress = self.rotation_in_progress.lock().await;
            *in_progress = false;
        }

        match &result {
            Ok(_) => {
                let duration = start_time.elapsed().as_millis() as u64;
                let old_version = self.secret_manager.active_version().unwrap_or(0);
                let new_version = self.secret_manager.active_version().unwrap_or(0);
                self.audit.rotation_succeeded(old_version, new_version, duration).await;
                info!("Rotation completed in {}ms", duration);
            }
            Err(e) => {
                self.audit.rotation_failed("rotation_error", &e.to_string()).await;
                error!("Rotation failed: {}", e);
            }
        }

        result
    }

    async fn do_rotate(&self, _force: bool) -> AgentResult<()> {
        // 1. Obtenir version actuelle
        let current_version = self.secret_manager.active_version().unwrap_or(0);
        info!("Starting rotation from version {}", current_version);

        // 2. Préparer requête
        let nonce = self.generate_nonce();
        let timestamp = Utc::now();
        
        // Créer données à signer
        let data_to_sign = format!(
            "{}{}{}{}",
            self.config.agent.id,
            current_version,
            timestamp.timestamp(),
            hex::encode(&nonce)
        );
        
        // Signer avec RSA-PSS
        let signature_bytes = self.crypto.sign_pss(data_to_sign.as_bytes())?;
        let signature = base64::encode(&signature_bytes);
        
        let request = RotateSecretRequest {
            agent_id: self.config.agent.id.clone(),
            current_version,
            timestamp,
            nonce: hex::encode(&nonce),
            signature,
        };

        // 3. Envoyer requête au serveur
        let response = self.send_rotation_request(&request).await?;

        // 4. Vérifier signature réponse
        // Note: Nécessite clé publique serveur dans config
        // Pour l'instant, on assume que la signature est valide si présente
        if !response.signature.is_empty() {
            // TODO: Charger clé publique serveur et vérifier signature
            debug!("Server signature present (verification not yet implemented)");
        }

        // 5. Déchiffrer nouveau secret avec RSA-OAEP
        let new_secret_encrypted = base64::decode(&response.new_secret_encrypted)
            .map_err(|e| AgentError::CryptoError(format!("Failed to decode secret: {}", e)))?;

        // Déchiffrer avec clé privée agent (RSA-OAEP)
        let new_secret_data = self.crypto.decrypt_oaep(&new_secret_encrypted, Some(b"license-secret"))?;

        // 6. Créer métadonnées nouveau secret
        let new_metadata = SecretMetadata {
            version: response.version,
            state: SecretState::Actif,
            valid_from: response.valid_from,
            valid_until: response.valid_until,
            grace_until: Some(response.grace_until),
            created_at: Utc::now(),
            last_used_at: None,
            rotation_source: RotationSource::Automatic,
            invalidation_reason: None,
        };

        let new_secret = Secret {
            data: new_secret_data,
            metadata: new_metadata,
        };

        // 7. Stocker nouveau secret
        self.secret_manager.store_secret(new_secret, response.version).await?;

        // 8. Passer ancien secret en GRACE
        if current_version > 0 {
            if let Err(e) = self.secret_manager.set_grace(current_version, response.grace_until).await {
                warn!("Failed to set old secret to GRACE: {}", e);
                // Ne pas échouer la rotation pour ça
            }
        }

        info!("Rotation completed: {} -> {}", current_version, response.version);

        Ok(())
    }

    async fn send_rotation_request(
        &self,
        request: &RotateSecretRequest,
    ) -> AgentResult<RotateSecretResponse> {
        let url = format!("{}/api/v1/rotate-secret", self.config.server.url);
        
        // Retry avec backoff exponentiel configurable
        let mut last_error = None;

        for attempt in 0..self.max_retries {
            match self.client
                .post(&url)
                .json(request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let rotate_response: RotateSecretResponse = response
                            .json()
                            .await
                            .map_err(|e| AgentError::NetworkError(format!("Failed to parse response: {}", e)))?;
                        return Ok(rotate_response);
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_default();
                        last_error = Some(AgentError::NetworkError(
                            format!("Server error {}: {}", status, error_text)
                        ));
                    }
                }
                Err(e) => {
                    last_error = Some(AgentError::NetworkError(format!("Request failed: {}", e)));
                }
            }

            if attempt < self.max_retries - 1 {
                // Backoff exponentiel : base_delay * 2^attempt
                let delay = Duration::from_secs(
                    self.base_retry_delay_seconds * 2_u64.pow(attempt)
                );
                warn!("Rotation request failed (attempt {}/{}), retrying in {:?}...", 
                      attempt + 1, self.max_retries, delay);
                tokio::time::sleep(delay).await;
            }
        }

        // Alerte si rotation échoue après tous les retries
        self.audit.error(
            "rotation_failed_after_retries",
            serde_json::json!({
                "max_retries": self.max_retries,
                "error": last_error.as_ref().map(|e| e.to_string())
            })
        ).await;

        Err(last_error.unwrap_or_else(|| {
            AgentError::RotationFailed(format!("Max retries ({}) exceeded", self.max_retries))
        }))
    }

    fn generate_nonce(&self) -> Vec<u8> {
        let mut nonce = vec![0u8; 16];
        rand::thread_rng().fill(&mut nonce[..]);
        nonce
    }

    /// Nettoie les secrets expirés
    pub async fn cleanup_expired(&self) -> AgentResult<usize> {
        self.secret_manager.cleanup_expired().await
    }
}
