use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_PATH: &str = "/etc/license-agent/config.toml";
const DEFAULT_STATE_PATH: &str = "/var/lib/license-agent/state.json";
const DEFAULT_AUDIT_LOG_PATH: &str = "/var/log/license-agent/audit.log";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub agent: AgentConfig,
    pub tpm: TpmConfig,
    pub management: ManagementConfig,
    pub degraded_mode: DegradedModeConfig,
    
    #[serde(skip)]
    config_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: String,
    pub cert_pin: String,
    pub client_cert: PathBuf,
    pub client_key: PathBuf,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    #[serde(default = "default_rotation_interval")]
    pub rotation_interval: u64,
    #[serde(default = "default_grace_period")]
    pub grace_period: u64,
    pub rotation_threshold_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmConfig {
    pub enabled: bool,
    pub fallback_encrypted_storage: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementConfig {
    pub allowed_uids: Vec<u32>,
    pub ipc_socket_path: Option<PathBuf>,
    pub api_port: Option<u16>,
    pub rate_limit_requests_per_minute: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradedModeConfig {
    pub enabled: bool,
    pub grace_period_days: u64,
    pub retry_interval_seconds: u64,
    pub auto_deactivate_on_reconnect: bool,
    pub alert_thresholds_hours: Vec<u64>,
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from_path(DEFAULT_CONFIG_PATH)
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;
        
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;
        
        config.config_path = path.to_path_buf();
        
        // Validation
        config.validate()?;
        
        Ok(config)
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn state_path(&self) -> PathBuf {
        PathBuf::from(DEFAULT_STATE_PATH)
    }

    pub fn audit_log_path(&self) -> PathBuf {
        PathBuf::from(DEFAULT_AUDIT_LOG_PATH)
    }

    pub fn ipc_socket_path(&self) -> PathBuf {
        self.management
            .ipc_socket_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("/var/run/license-agent.sock"))
    }

    fn validate(&self) -> Result<()> {
        // Validation URLs
        if !self.server.url.starts_with("https://") {
            anyhow::bail!("Server URL must use HTTPS");
        }

        // Validation chemins
        if !self.server.client_cert.exists() {
            anyhow::bail!("Client certificate not found: {}", self.server.client_cert.display());
        }

        if !self.server.client_key.exists() {
            anyhow::bail!("Client key not found: {}", self.server.client_key.display());
        }

        // Validation intervalles
        if self.agent.rotation_interval == 0 {
            anyhow::bail!("Rotation interval must be > 0");
        }

        if self.agent.grace_period == 0 {
            anyhow::bail!("Grace period must be > 0");
        }

        Ok(())
    }
}

fn default_rotation_interval() -> u64 {
    86400 // 24 heures
}

fn default_grace_period() -> u64 {
    604800 // 7 jours
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: "https://license-server.example.com".to_string(),
            cert_pin: String::new(),
            client_cert: PathBuf::from("/etc/license-agent/client.pem"),
            client_key: PathBuf::from("/etc/license-agent/client.key"),
            timeout_seconds: Some(30),
        }
    }
}

impl Default for DegradedModeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            grace_period_days: 7,
            retry_interval_seconds: 300, // 5 minutes
            auto_deactivate_on_reconnect: true,
            alert_thresholds_hours: vec![24, 72, 144],
        }
    }
}
