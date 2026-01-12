use crate::audit::AuditLogger;
use crate::config::Config;
use crate::crypto::CryptoManager;
use crate::ipc::IpcServer;
use crate::license::LicenseValidator;
use crate::rotation::RotationManager;
use crate::secret::SecretManager;
use crate::tpm::TpmManager;
use crate::types::{AgentError, AgentResult, DegradedModeStatus, SystemStatus};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Moteur principal
pub struct CoreEngine {
    config: Arc<Config>,
    tpm: Arc<TpmManager>,
    secret_manager: Arc<SecretManager>,
    validator: Arc<LicenseValidator>,
    rotation_manager: Arc<RotationManager>,
    audit: Arc<AuditLogger>,
    ipc_server: Option<Arc<IpcServer>>,
    degraded_mode: Arc<RwLock<DegradedModeState>>,
    shutdown: Arc<tokio::sync::Notify>,
}

struct DegradedModeState {
    active: bool,
    activated_at: Option<DateTime<Utc>>,
    grace_period_end: Option<DateTime<Utc>>,
}

impl CoreEngine {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let config = Arc::new(config);

        // Initialiser TPM
        let tpm = Arc::new(TpmManager::new(config.tpm.enabled)?);
        info!("TPM manager initialized (available: {})", tpm.is_available());

        // Initialiser Secret Manager
        let secret_manager = Arc::new(SecretManager::new(
            Arc::clone(&tpm),
            config.state_path(),
        ));

        // Charger état
        secret_manager.load_state().await?;
        info!("Secret manager initialized");

        // Initialiser Audit Logger
        let audit = Arc::new(AuditLogger::new(&config).await?);

        // Initialiser License Validator
        let validator = Arc::new(LicenseValidator::new(Arc::clone(&secret_manager)));

        // Initialiser Crypto Manager
        // Charger ou générer clés RSA agent
        let crypto = Arc::new(
            if config.server.client_key.exists() {
                CryptoManager::from_pem_files(
                    &config.server.client_key.to_string_lossy(),
                    None,
                )?
            } else {
                // Générer nouvelles clés (première exécution)
                let (private_key, public_key) = CryptoManager::generate_keys()?;
                CryptoManager::new(private_key, public_key)
            }
        );

        // Initialiser Rotation Manager
        let rotation_manager = Arc::new(RotationManager::new(
            Arc::clone(&config),
            Arc::clone(&secret_manager),
            Arc::clone(&audit),
            Arc::clone(&crypto),
        )?);

        // État mode dégradé
        let degraded_mode = Arc::new(RwLock::new(DegradedModeState {
            active: false,
            activated_at: None,
            grace_period_end: None,
        }));

        Ok(Self {
            config,
            tpm,
            secret_manager,
            validator,
            rotation_manager,
            audit,
            ipc_server: None,
            degraded_mode,
            shutdown: Arc::new(tokio::sync::Notify::new()),
        })
    }

    /// Démarre le moteur
    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Starting core engine...");

        // Vérifier si rotation nécessaire au démarrage
        if self.rotation_manager.check_rotation_needed().await {
            info!("Rotation needed at startup");
            if let Err(e) = self.rotation_manager.rotate(false).await {
                warn!("Failed to rotate at startup: {}", e);
                // Activer mode dégradé si pas de secret
                if self.secret_manager.active_version().is_none() {
                    self.activate_degraded_mode("No active secret at startup").await;
                }
            }
        }

        // Démarrer serveur IPC
        let ipc_server = Arc::new(
            IpcServer::new(
                self.config.ipc_socket_path(),
                Arc::clone(&self.validator),
                self.config.management.allowed_uids.clone(),
            )
            .await?,
        );

        // Démarrer serveur IPC en arrière-plan
        let ipc_server_clone = Arc::clone(&ipc_server);
        tokio::spawn(async move {
            if let Err(e) = ipc_server_clone.run().await {
                error!("IPC server error: {}", e);
            }
        });

        // Démarrer tâches périodiques
        self.start_periodic_tasks();

        info!("Core engine started successfully");

        Ok(())
    }

    fn start_periodic_tasks(&self) {
        let rotation_manager = Arc::clone(&self.rotation_manager);
        let secret_manager = Arc::clone(&self.secret_manager);
        let degraded_mode = Arc::clone(&self.degraded_mode);
        let audit = Arc::clone(&self.audit);
        let config = Arc::clone(&self.config);
        let shutdown = Arc::clone(&self.shutdown);

        // Tâche de rotation périodique
        let secret_manager_clone = Arc::clone(&secret_manager);
        let degraded_mode_clone = Arc::clone(&degraded_mode);
        let config_rotation = Arc::clone(&config);
        let shutdown_rotation = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Toutes les heures

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if rotation_manager.check_rotation_needed().await {
                            if let Err(e) = rotation_manager.rotate(false).await {
                                warn!("Periodic rotation failed: {}", e);
                                // Activer mode dégradé si pas de secret actif
                                if secret_manager_clone.active_version().is_none() {
                                    let mut state = degraded_mode_clone.write().await;
                                    if !state.active {
                                        state.active = true;
                                        state.activated_at = Some(Utc::now());
                                        let grace_period_days = config_rotation.degraded_mode.grace_period_days;
                                        state.grace_period_end = Some(Utc::now() + chrono::Duration::days(grace_period_days as i64));
                                    }
                                }
                            }
                        }
                    }
                    _ = shutdown_rotation.notified() => {
                        break;
                    }
                }
            }
        });

        // Tâche de nettoyage
        let shutdown_cleanup = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Toutes les heures

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = secret_manager.cleanup_expired().await {
                            warn!("Cleanup failed: {}", e);
                        }
                    }
                    _ = shutdown_cleanup.notified() => {
                        break;
                    }
                }
            }
        });

        // Tâche de vérification mode dégradé avec retry rotation
        let rotation_manager_retry = Arc::clone(&self.rotation_manager);
        let audit_clone = Arc::clone(&audit);
        let degraded_mode_retry = Arc::clone(&degraded_mode);
        let config_retry = Arc::clone(&config);
        let shutdown_retry = Arc::clone(&shutdown);
        tokio::spawn(async move {
            let retry_interval = tokio::time::Duration::from_secs(
                config_retry.degraded_mode.retry_interval_seconds
            );
            let mut interval = tokio::time::interval(retry_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let state = degraded_mode_retry.read().await;
                        if state.active {
                            // Vérifier expiration grace period
                            if let Some(grace_end) = state.grace_period_end {
                                if Utc::now() > grace_end {
                                    warn!("Grace period expired, degraded mode should be deactivated");
                                }
                            }
                            
                            // Retry rotation en mode dégradé
                            drop(state);
                            if rotation_manager_retry.check_rotation_needed().await {
                                if let Err(e) = rotation_manager_retry.rotate(false).await {
                                    debug!("Rotation retry in degraded mode failed: {}", e);
                                } else {
                                    // Rotation réussie, désactiver mode dégradé
                                    let mut state = degraded_mode_retry.write().await;
                                    state.active = false;
                                    state.activated_at = None;
                                    state.grace_period_end = None;
                                    audit_clone.info(
                                        "degraded_mode_deactivated_auto",
                                        serde_json::json!({"reason": "rotation_successful"})
                                    ).await;
                                }
                            }
                        }
                    }
                    _ = shutdown_retry.notified() => {
                        break;
                    }
                }
            }
        });

        // Tâche d'alertes progressives mode dégradé
        let degraded_mode_alerts = Arc::clone(&self.degraded_mode);
        let config_alerts = Arc::clone(&self.config);
        let audit_alerts = Arc::clone(&self.audit);
        let shutdown_alerts = Arc::clone(&self.shutdown);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Toutes les heures

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let state = degraded_mode_alerts.read().await;
                        if state.active {
                            if let Some(activated_at) = state.activated_at {
                                let duration = Utc::now().signed_duration_since(activated_at);
                                let hours = duration.num_hours();
                                
                                // Alertes progressives selon seuils configurés
                                for threshold in &config_alerts.degraded_mode.alert_thresholds_hours {
                                    if hours == *threshold as i64 {
                                        audit_alerts.warning(
                                            "degraded_mode_alert",
                                            serde_json::json!({
                                                "duration_hours": hours,
                                                "threshold": threshold
                                            })
                                        ).await;
                                    }
                                }
                            }
                        }
                    }
                    _ = shutdown_alerts.notified() => {
                        break;
                    }
                }
            }
        });
    }

    async fn activate_degraded_mode(&self, reason: &str) {
        let mut state = self.degraded_mode.write().await;
        if !state.active {
            state.active = true;
            state.activated_at = Some(Utc::now());
            
            // Calculer grace period end
            let grace_period_days = self.config.degraded_mode.grace_period_days;
            state.grace_period_end = Some(Utc::now() + chrono::Duration::days(grace_period_days as i64));
            
            self.audit.degraded_mode_activated(reason).await;
            warn!("Degraded mode activated: {}", reason);
        }
    }

    /// Arrêt gracieux
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        info!("Shutting down core engine...");
        
        // Notifier toutes les tâches
        self.shutdown.notify_waiters();
        
        // Sauvegarder état
        if let Err(e) = self.secret_manager.save_state().await {
            error!("Failed to save state during shutdown: {}", e);
        }

        info!("Core engine shut down");
        Ok(())
    }

    /// Obtient le statut du système
    pub async fn get_status(&self) -> AgentResult<SystemStatus> {
        let active_version = self.secret_manager.active_version();
        let active_secret = if let Some(version) = active_version {
            self.secret_manager.get_metadata(version).map(|metadata| {
                crate::types::SecretInfo {
                    version: metadata.version,
                    state: metadata.state,
                    valid_from: metadata.valid_from,
                    valid_until: metadata.valid_until,
                    grace_until: metadata.grace_until,
                    remaining_seconds: Some(
                        metadata
                            .valid_until
                            .signed_duration_since(Utc::now())
                            .num_seconds()
                            .max(0)
                    ),
                }
            })
        } else {
            None
        };

        let grace_secrets: Vec<_> = self
            .secret_manager
            .list_versions()
            .iter()
            .filter_map(|v| {
                self.secret_manager.get_metadata(*v).and_then(|m| {
                    if m.state == crate::types::SecretState::Grace {
                        Some(crate::types::SecretInfo {
                            version: m.version,
                            state: m.state,
                            valid_from: m.valid_from,
                            valid_until: m.valid_until,
                            grace_until: m.grace_until,
                            remaining_seconds: m.grace_until.and_then(|g| {
                                g.signed_duration_since(Utc::now())
                                    .num_seconds()
                                    .max(0)
                                    .try_into()
                                    .ok()
                            }),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        let degraded_state = self.degraded_mode.read().await;
        let degraded_mode_status = DegradedModeStatus {
            active: degraded_state.active,
            activated_at: degraded_state.activated_at,
            duration_seconds: degraded_state.activated_at.map(|a| {
                Utc::now().signed_duration_since(a).num_seconds()
            }),
            grace_period_end: degraded_state.grace_period_end,
            remaining_seconds: degraded_state.grace_period_end.map(|g| {
                g.signed_duration_since(Utc::now()).num_seconds().max(0)
            }),
        };

        let next_rotation = active_secret.as_ref().and_then(|s| {
            s.valid_until.signed_duration_since(Utc::now()).num_seconds().try_into().ok()
                .map(|secs: i64| Utc::now() + chrono::Duration::seconds(secs))
        });

        Ok(SystemStatus {
            active_secret,
            grace_secrets,
            tpm_status: self.tpm.get_status(),
            license_status: self.validator.get_stats().await,
            degraded_mode: degraded_mode_status,
            next_rotation,
        })
    }
}
