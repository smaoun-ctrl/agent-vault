use crate::config::Config;
use crate::types::{AgentError, SystemStatus};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Parser)]
#[command(name = "license-agent-cli")]
#[command(about = "CLI de gestion pour License Secret Agent")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Chemin vers le certificat client pour authentification
    #[arg(long, default_value = "/etc/license-agent/admin.crt")]
    pub cert: Option<PathBuf>,

    /// Chemin vers la clé privée client
    #[arg(long, default_value = "/etc/license-agent/admin.key")]
    pub key: Option<PathBuf>,

    /// Token d'authentification (alternative au certificat)
    #[arg(long)]
    pub token: Option<String>,

    /// Chemin vers le socket IPC
    #[arg(long, default_value = "/var/run/license-agent.sock")]
    pub socket: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Affiche le statut du système
    Status,
    
    /// Force une rotation du secret
    Rotate {
        /// Forcer la rotation même si non nécessaire
        #[arg(long)]
        force: bool,
    },
    
    /// Invalide un secret
    Invalidate {
        /// Version du secret à invalider
        version: u64,
        /// Raison de l'invalidation
        #[arg(long)]
        reason: Option<String>,
        /// Confirmer l'invalidation (requis pour secret ACTIF)
        #[arg(long)]
        confirm: bool,
    },
    
    /// Affiche les logs d'audit
    Logs {
        /// Nombre de lignes à afficher
        #[arg(long, default_value = "100")]
        tail: usize,
        /// Filtrer par événement
        #[arg(long)]
        event: Option<String>,
        /// Filtrer par niveau
        #[arg(long)]
        level: Option<String>,
        /// Depuis cette date (ISO 8601)
        #[arg(long)]
        since: Option<String>,
    },
    
    /// Affiche les métriques
    Metrics,
    
    /// Gère le mode dégradé
    DegradedMode {
        /// Activer le mode dégradé
        #[arg(long)]
        enable: bool,
        /// Désactiver le mode dégradé
        #[arg(long)]
        disable: bool,
        /// Raison (requis pour activation)
        #[arg(long)]
        reason: Option<String>,
    },
    
    /// Statut TPM
    TpmStatus,
    
    /// Réinitialise complètement le système
    Reset {
        /// Confirmer la réinitialisation
        #[arg(long)]
        confirm: bool,
        /// Confirmer à nouveau
        #[arg(long)]
        confirm_again: bool,
    },
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        // Authentification
        self.authenticate().await?;

        // Exécuter commande
        match self.command {
            Commands::Status => self.cmd_status().await,
            Commands::Rotate { force } => self.cmd_rotate(force).await,
            Commands::Invalidate { version, reason, confirm } => {
                self.cmd_invalidate(version, reason, confirm).await
            }
            Commands::Logs { tail, event, level, since } => {
                self.cmd_logs(tail, event, level, since).await
            }
            Commands::Metrics => self.cmd_metrics().await,
            Commands::DegradedMode { enable, disable, reason } => {
                self.cmd_degraded_mode(enable, disable, reason).await
            }
            Commands::TpmStatus => self.cmd_tpm_status().await,
            Commands::Reset { confirm, confirm_again } => {
                self.cmd_reset(confirm, confirm_again).await
            }
        }
    }

    async fn authenticate(&self) -> Result<()> {
        // Vérifier certificat ou token
        if let Some(token) = &self.token {
            // Vérifier token
            self.verify_token(token).await?;
        } else if let Some(cert_path) = &self.cert {
            // Vérifier certificat
            self.verify_certificate(cert_path).await?;
        } else {
            anyhow::bail!("Authentification requise: --cert ou --token");
        }
        Ok(())
    }

    async fn verify_token(&self, _token: &str) -> Result<()> {
        // TODO: Implémenter vérification token
        Ok(())
    }

    async fn verify_certificate(&self, _cert_path: &PathBuf) -> Result<()> {
        // TODO: Implémenter vérification certificat
        Ok(())
    }

    async fn cmd_status(&self) -> Result<()> {
        let status = self.send_request("status", serde_json::json!({})).await?;
        let status: SystemStatus = serde_json::from_value(status)?;
        
        println!("=== Statut License Secret Agent ===\n");
        
        if let Some(active) = &status.active_secret {
            println!("Secret ACTIF:");
            println!("  Version: {}", active.version);
            println!("  État: {:?}", active.state);
            println!("  Valide depuis: {}", active.valid_from);
            println!("  Valide jusqu'à: {}", active.valid_until);
            if let Some(remaining) = active.remaining_seconds {
                println!("  Temps restant: {} secondes", remaining);
            }
        } else {
            println!("Aucun secret ACTIF");
        }
        
        if !status.grace_secrets.is_empty() {
            println!("\nSecrets GRACE:");
            for secret in &status.grace_secrets {
                println!("  Version {}: jusqu'à {:?}", secret.version, secret.grace_until);
            }
        }
        
        println!("\nTPM: {}", if status.tpm_status.available { "Disponible" } else { "Indisponible" });
        println!("Mode dégradé: {}", if status.degraded_mode.active { "Actif" } else { "Inactif" });
        
        if let Some(next) = status.next_rotation {
            println!("Prochaine rotation: {}", next);
        }
        
        Ok(())
    }

    async fn cmd_rotate(&self, force: bool) -> Result<()> {
        let result = self.send_request("rotate", serde_json::json!({ "force": force })).await?;
        println!("Rotation: {}", result);
        Ok(())
    }

    async fn cmd_invalidate(&self, version: u64, reason: Option<String>, confirm: bool) -> Result<()> {
        if !confirm {
            anyhow::bail!("Confirmation requise pour invalider un secret (--confirm)");
        }
        
        let result = self.send_request("invalidate", serde_json::json!({
            "version": version,
            "reason": reason
        })).await?;
        
        println!("Secret {} invalidé: {}", version, result);
        Ok(())
    }

    async fn cmd_logs(&self, tail: usize, event: Option<String>, level: Option<String>, since: Option<String>) -> Result<()> {
        let result = self.send_request("logs", serde_json::json!({
            "tail": tail,
            "event": event,
            "level": level,
            "since": since
        })).await?;
        
        println!("{}", result);
        Ok(())
    }

    async fn cmd_metrics(&self) -> Result<()> {
        let result = self.send_request("metrics", serde_json::json!({})).await?;
        println!("{}", result);
        Ok(())
    }

    async fn cmd_degraded_mode(&self, enable: bool, disable: bool, reason: Option<String>) -> Result<()> {
        if enable && disable {
            anyhow::bail!("Ne peut pas activer et désactiver simultanément");
        }
        
        if enable && reason.is_none() {
            anyhow::bail!("Raison requise pour activer le mode dégradé (--reason)");
        }
        
        let result = self.send_request("degraded_mode", serde_json::json!({
            "enable": enable,
            "disable": disable,
            "reason": reason
        })).await?;
        
        println!("Mode dégradé: {}", result);
        Ok(())
    }

    async fn cmd_tpm_status(&self) -> Result<()> {
        let result = self.send_request("tpm_status", serde_json::json!({})).await?;
        println!("{}", result);
        Ok(())
    }

    async fn cmd_reset(&self, confirm: bool, confirm_again: bool) -> Result<()> {
        if !confirm || !confirm_again {
            anyhow::bail!("Double confirmation requise pour réinitialisation (--confirm --confirm-again)");
        }
        
        let result = self.send_request("reset", serde_json::json!({})).await?;
        println!("Réinitialisation: {}", result);
        Ok(())
    }

    async fn send_request(&self, command: &str, data: serde_json::Value) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(&self.socket).await?;
        
        let request = serde_json::json!({
            "command": command,
            "data": data
        });
        
        let request_bytes = serde_json::to_vec(&request)?;
        stream.write_all(&(request_bytes.len() as u32).to_be_bytes()).await?;
        stream.write_all(&request_bytes).await?;
        
        // Lire réponse
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        let mut response_bytes = vec![0u8; len];
        stream.read_exact(&mut response_bytes).await?;
        
        let response: serde_json::Value = serde_json::from_slice(&response_bytes)?;
        Ok(response)
    }
}
