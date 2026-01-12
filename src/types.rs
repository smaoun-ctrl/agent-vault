use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// État d'un secret
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SecretState {
    Absent,
    Actif,
    Grace,
    Invalide,
}

/// Métadonnées d'un secret
/// Note: Les métadonnées ne contiennent pas de données sensibles, donc pas besoin de ZeroizeOnDrop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    pub version: u64,
    pub state: SecretState,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub grace_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub rotation_source: RotationSource,
    pub invalidation_reason: Option<String>,
}

/// Source de rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RotationSource {
    Automatic,
    Manual,
    Recovery,
}

/// Secret avec métadonnées
/// Note: Seul le champ `data` est zéroisé à la destruction, pas les métadonnées
#[derive(ZeroizeOnDrop)]
pub struct Secret {
    #[zeroize(on_drop)]
    pub data: Vec<u8>,
    #[zeroize(skip)]
    pub metadata: SecretMetadata,
}

/// Informations de licence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub license_id: String,
    pub customer_id: String,
    pub features: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Résultat de validation de licence
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub features: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub error: Option<String>,
}

/// Requête de validation
#[derive(Debug, Deserialize)]
pub struct ValidateLicenseRequest {
    pub license_token: Vec<u8>,
    pub nonce: [u8; 16],
}

/// Réponse de validation
#[derive(Debug, Serialize)]
pub struct ValidateLicenseResponse {
    pub result: ValidationResult,
}

/// État du système
#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub active_secret: Option<SecretInfo>,
    pub grace_secrets: Vec<SecretInfo>,
    pub tpm_status: TpmStatus,
    pub license_status: LicenseStatus,
    pub degraded_mode: DegradedModeStatus,
    pub next_rotation: Option<DateTime<Utc>>,
}

/// Informations sur un secret (sans le secret lui-même)
#[derive(Debug, Clone, Serialize)]
pub struct SecretInfo {
    pub version: u64,
    pub state: SecretState,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub grace_until: Option<DateTime<Utc>>,
    pub remaining_seconds: Option<i64>,
}

/// État TPM
#[derive(Debug, Serialize)]
pub struct TpmStatus {
    pub available: bool,
    pub version: Option<String>,
    pub manufacturer: Option<String>,
    pub firmware_version: Option<String>,
    pub keys_loaded: usize,
    pub nv_space_used: Option<f64>,
}

/// État de la licence
#[derive(Debug, Serialize)]
pub struct LicenseStatus {
    pub last_validation: Option<DateTime<Utc>>,
    pub total_validations: u64,
    pub successful_validations: u64,
    pub failed_validations: u64,
    pub last_error: Option<String>,
}

/// État du mode dégradé
#[derive(Debug, Serialize)]
pub struct DegradedModeStatus {
    pub active: bool,
    pub activated_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub grace_period_end: Option<DateTime<Utc>>,
    pub remaining_seconds: Option<i64>,
}

/// Erreurs du système
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Secret not found: version {0}")]
    SecretNotFound(u64),
    
    #[error("Secret expired: version {0}")]
    SecretExpired(u64),
    
    #[error("Secret invalid: {0}")]
    SecretInvalid(String),
    
    #[error("License validation failed: {0}")]
    LicenseValidationFailed(String),
    
    #[error("TPM error: {0}")]
    TpmError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("IPC error: {0}")]
    IpcError(String),
    
    #[error("Rotation failed: {0}")]
    RotationFailed(String),
    
    #[error("Cryptographic error: {0}")]
    CryptoError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type AgentResult<T> = Result<T, AgentError>;
