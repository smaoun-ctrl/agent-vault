use crate::config::Config;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Événement d'audit
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event: String,
    pub level: AuditLevel,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Logger d'audit
pub struct AuditLogger {
    log_path: PathBuf,
    file: Arc<Mutex<tokio::fs::File>>,
}

impl AuditLogger {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let log_path = config.audit_log_path();

        // Créer répertoire si nécessaire
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Ouvrir fichier en mode append
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await?;

        info!("Audit logger initialized: {}", log_path.display());

        Ok(Self {
            log_path,
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Log un événement
    pub async fn log(&self, event: &str, level: AuditLevel, data: serde_json::Value) {
        let audit_event = AuditEvent {
            timestamp: Utc::now(),
            event: event.to_string(),
            level,
            data,
        };

        // Sérialiser en JSON
        let json = match serde_json::to_string(&audit_event) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to serialize audit event: {}", e);
                return;
            }
        };

        // Écrire dans le fichier
        let mut file = self.file.lock().await;
        if let Err(e) = file.write_all(json.as_bytes()).await {
            error!("Failed to write audit log: {}", e);
            return;
        }
        if let Err(e) = file.write_all(b"\n").await {
            error!("Failed to write newline: {}", e);
            return;
        }
        if let Err(e) = file.flush().await {
            error!("Failed to flush audit log: {}", e);
        }

        debug!("Audit event logged: {} ({:?})", event, level);
    }

    /// Log info
    pub async fn info(&self, event: &str, data: serde_json::Value) {
        self.log(event, AuditLevel::Info, data).await;
    }

    /// Log warning
    pub async fn warning(&self, event: &str, data: serde_json::Value) {
        self.log(event, AuditLevel::Warning, data).await;
    }

    /// Log error
    pub async fn error(&self, event: &str, data: serde_json::Value) {
        self.log(event, AuditLevel::Error, data).await;
    }

    /// Log critical
    pub async fn critical(&self, event: &str, data: serde_json::Value) {
        self.log(event, AuditLevel::Critical, data).await;
    }

    /// Log rotation réussie
    pub async fn rotation_succeeded(
        &self,
        old_version: u64,
        new_version: u64,
        duration_ms: u64,
    ) {
        self.info(
            "rotation_succeeded",
            serde_json::json!({
                "old_version": old_version,
                "new_version": new_version,
                "duration_ms": duration_ms,
            }),
        )
        .await;
    }

    /// Log rotation échouée
    pub async fn rotation_failed(&self, reason: &str, error: &str) {
        self.error(
            "rotation_failed",
            serde_json::json!({
                "reason": reason,
                "error": error,
            }),
        )
        .await;
    }

    /// Log validation licence
    pub async fn license_validated(&self, license_id: &str, version: u64, result: &str) {
        self.info(
            "license_validated",
            serde_json::json!({
                "license_id": license_id,
                "secret_version": version,
                "result": result,
            }),
        )
        .await;
    }

    /// Log activation mode dégradé
    pub async fn degraded_mode_activated(&self, reason: &str) {
        self.warning(
            "degraded_mode_activated",
            serde_json::json!({
                "reason": reason,
            }),
        )
        .await;
    }

    /// Log désactivation mode dégradé
    pub async fn degraded_mode_deactivated(&self, duration_seconds: i64) {
        self.info(
            "degraded_mode_deactivated",
            serde_json::json!({
                "duration_seconds": duration_seconds,
            }),
        )
        .await;
    }

    /// Log invalidation secret
    pub async fn secret_invalidated(&self, version: u64, reason: Option<&str>) {
        self.warning(
            "secret_invalidated",
            serde_json::json!({
                "version": version,
                "reason": reason,
            }),
        )
        .await;
    }
}
