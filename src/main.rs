use anyhow::Result;
use license_secret_agent::config::Config;
use license_secret_agent::core::CoreEngine;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialisation logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "license_secret_agent=info".into()),
        )
        .with_target(false)
        .init();

    info!("License Secret Agent starting...");

    // Chargement configuration
    let config = Config::load()?;
    info!("Configuration loaded from {}", config.config_path().display());

    // Création moteur principal
    let engine = Arc::new(CoreEngine::new(config).await?);
    info!("Core engine initialized");

    // Démarrage du moteur
    if let Err(e) = engine.start().await {
        error!("Failed to start core engine: {}", e);
        return Err(e);
    }

    info!("License Secret Agent started successfully");

    // Attente signal d'arrêt
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received");

    // Arrêt gracieux
    engine.shutdown().await?;
    info!("License Secret Agent stopped");

    Ok(())
}
