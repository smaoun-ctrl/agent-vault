use prometheus::{
    register_counter_with_registry, register_gauge_with_registry, register_histogram_with_registry,
    Counter, Gauge, Histogram, Registry,
};
use std::sync::Arc;

/// Métriques Prometheus
pub struct Metrics {
    pub secrets_active: Gauge,
    pub secrets_grace: Gauge,
    pub secrets_invalidated: Gauge,
    pub rotations_total: Counter,
    pub rotations_failed: Counter,
    pub rotations_duration: Histogram,
    pub validations_total: Counter,
    pub validations_successful: Counter,
    pub validations_failed: Counter,
    pub validation_duration: Histogram,
    pub degraded_mode_active: Gauge,
    pub degraded_mode_duration: Histogram,
    pub tpm_available: Gauge,
    pub last_rotation_timestamp: Gauge,
}

impl Metrics {
    pub fn new(registry: Arc<Registry>) -> anyhow::Result<Self> {
        Ok(Self {
            secrets_active: register_gauge_with_registry!(
                "license_agent_secrets_active",
                "Number of active secrets",
                registry
            )?,
            secrets_grace: register_gauge_with_registry!(
                "license_agent_secrets_grace",
                "Number of secrets in grace period",
                registry
            )?,
            secrets_invalidated: register_gauge_with_registry!(
                "license_agent_secrets_invalidated",
                "Number of invalidated secrets",
                registry
            )?,
            rotations_total: register_counter_with_registry!(
                "license_agent_rotations_total",
                "Total number of rotations",
                registry
            )?,
            rotations_failed: register_counter_with_registry!(
                "license_agent_rotations_failed",
                "Total number of failed rotations",
                registry
            )?,
            rotations_duration: register_histogram_with_registry!(
                "license_agent_rotation_duration_seconds",
                "Duration of rotation in seconds",
                registry
            )?,
            validations_total: register_counter_with_registry!(
                "license_agent_validations_total",
                "Total number of license validations",
                registry
            )?,
            validations_successful: register_counter_with_registry!(
                "license_agent_validations_successful",
                "Total number of successful validations",
                registry
            )?,
            validations_failed: register_counter_with_registry!(
                "license_agent_validations_failed",
                "Total number of failed validations",
                registry
            )?,
            validation_duration: register_histogram_with_registry!(
                "license_agent_validation_duration_seconds",
                "Duration of validation in seconds",
                registry
            )?,
            degraded_mode_active: register_gauge_with_registry!(
                "license_agent_degraded_mode_active",
                "Degraded mode active (1) or inactive (0)",
                registry
            )?,
            degraded_mode_duration: register_histogram_with_registry!(
                "license_agent_degraded_mode_duration_seconds",
                "Duration of degraded mode in seconds",
                registry
            )?,
            tpm_available: register_gauge_with_registry!(
                "license_agent_tpm_available",
                "TPM available (1) or unavailable (0)",
                registry
            )?,
            last_rotation_timestamp: register_gauge_with_registry!(
                "license_agent_last_rotation_timestamp",
                "Timestamp of last rotation",
                registry
            )?,
        })
    }

    /// Met à jour les métriques de secrets
    pub fn update_secrets(&self, active: usize, grace: usize, invalidated: usize) {
        self.secrets_active.set(active as f64);
        self.secrets_grace.set(grace as f64);
        self.secrets_invalidated.set(invalidated as f64);
    }

    /// Enregistre une rotation réussie
    pub fn record_rotation_success(&self, duration_seconds: f64) {
        self.rotations_total.inc();
        self.rotations_duration.observe(duration_seconds);
        self.last_rotation_timestamp.set(chrono::Utc::now().timestamp() as f64);
    }

    /// Enregistre une rotation échouée
    pub fn record_rotation_failure(&self) {
        self.rotations_failed.inc();
    }

    /// Enregistre une validation
    pub fn record_validation(&self, success: bool, duration_seconds: f64) {
        self.validations_total.inc();
        if success {
            self.validations_successful.inc();
        } else {
            self.validations_failed.inc();
        }
        self.validation_duration.observe(duration_seconds);
    }

    /// Met à jour le mode dégradé
    pub fn update_degraded_mode(&self, active: bool, duration_seconds: Option<f64>) {
        self.degraded_mode_active.set(if active { 1.0 } else { 0.0 });
        if let Some(duration) = duration_seconds {
            self.degraded_mode_duration.observe(duration);
        }
    }

    /// Met à jour le statut TPM
    pub fn update_tpm_status(&self, available: bool) {
        self.tpm_available.set(if available { 1.0 } else { 0.0 });
    }
}

/// Crée un registre Prometheus et les métriques
pub fn create_metrics() -> anyhow::Result<(Arc<Registry>, Arc<Metrics>)> {
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(Metrics::new(Arc::clone(&registry))?);
    Ok((registry, metrics))
}
