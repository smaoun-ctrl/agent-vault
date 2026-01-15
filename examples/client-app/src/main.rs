use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// Application cliente exemple qui utilise l'agent pour valider des licences
#[derive(Debug)]
struct LicenseClient {
    socket_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidateRequest {
    license_token: Vec<u8>,
    nonce: [u8; 16],
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidateResponse {
    result: ValidationResult,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidationResult {
    valid: bool,
    expires_at: Option<String>,
    features: Vec<String>,
    metadata: std::collections::HashMap<String, String>,
    error: Option<String>,
}

impl LicenseClient {
    fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /// Valide une licence via l'agent
    async fn validate_license(&self, license_token: Vec<u8>) -> Result<ValidationResult> {
        // Générer un nonce
        let nonce = self.generate_nonce();

        // Créer la requête
        let request = ValidateRequest {
            license_token,
            nonce,
        };

        // Envoyer à l'agent via IPC
        let response: ValidateResponse = self.send_ipc_request("validate", &request).await?;

        Ok(response.result)
    }

    /// Envoie une requête IPC à l'agent
    async fn send_ipc_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        command: &str,
        data: &T,
    ) -> Result<R> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .context("Failed to connect to agent socket")?;

        let request = serde_json::json!({
            "command": command,
            "data": data
        });

        let request_bytes = serde_json::to_vec(&request)?;

        // Envoyer la longueur puis les données
        stream.write_all(&(request_bytes.len() as u32).to_be_bytes()).await?;
        stream.write_all(&request_bytes).await?;

        // Lire la réponse
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut response_bytes = vec![0u8; len];
        stream.read_exact(&mut response_bytes).await?;

        let response: serde_json::Value = serde_json::from_slice(&response_bytes)?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("Agent error: {}", error);
        }

        let result: R = serde_json::from_value(
            response.get("data").cloned()
                .ok_or_else(|| anyhow::anyhow!("Missing data in response"))?
        )?;

        Ok(result)
    }

    fn generate_nonce(&self) -> [u8; 16] {
        use rand::Rng;
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill(&mut nonce);
        nonce
    }
}

/// Application exemple
struct ExampleApp {
    license_client: LicenseClient,
    license_token: Option<Vec<u8>>,
}

impl ExampleApp {
    fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            license_client: LicenseClient::new(socket_path),
            license_token: None,
        }
    }

    /// Charge une licence depuis un fichier
    async fn load_license(&mut self, path: &str) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose};
        
        let license_b64 = std::fs::read_to_string(path)
            .context("Failed to read license file")?;
        
        let license_token = general_purpose::STANDARD.decode(license_b64.trim())?;
        self.license_token = Some(license_token);
        
        println!("✓ Licence chargée depuis {}", path);
        Ok(())
    }

    /// Valide la licence actuelle
    async fn validate_license(&self) -> Result<()> {
        let license_token = self.license_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Aucune licence chargée"))?;

        println!("Validation de la licence...");
        let result = self.license_client.validate_license(license_token.clone()).await?;

        if result.valid {
            println!("✓ Licence VALIDE");
            if let Some(expires_at) = result.expires_at {
                println!("  Expire le: {}", expires_at);
            }
            if !result.features.is_empty() {
                println!("  Fonctionnalités: {}", result.features.join(", "));
            }
            if !result.metadata.is_empty() {
                println!("  Métadonnées:");
                for (key, value) in &result.metadata {
                    println!("    {}: {}", key, value);
                }
            }
        } else {
            println!("✗ Licence INVALIDE");
            if let Some(error) = result.error {
                println!("  Erreur: {}", error);
            }
        }

        Ok(())
    }

    /// Simule l'utilisation d'une fonctionnalité
    async fn use_feature(&self, feature: &str) -> Result<()> {
        let license_token = self.license_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Aucune licence chargée"))?;

        // Valider la licence avant d'utiliser la fonctionnalité
        let result = self.license_client.validate_license(license_token.clone()).await?;

        if !result.valid {
            anyhow::bail!("Licence invalide, fonctionnalité '{}' non disponible", feature);
        }

        if !result.features.contains(&feature.to_string()) {
            anyhow::bail!("Fonctionnalité '{}' non incluse dans la licence", feature);
        }

        println!("✓ Utilisation de la fonctionnalité: {}", feature);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("client_app=info")
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!();
        eprintln!("Commandes:");
        eprintln!("  load <license_file>     Charge une licence depuis un fichier");
        eprintln!("  validate                Valide la licence actuelle");
        eprintln!("  use <feature>           Utilise une fonctionnalité (nécessite licence valide)");
        eprintln!();
        eprintln!("Exemple:");
        eprintln!("  {} load license.txt", args[0]);
        eprintln!("  {} validate", args[0]);
        eprintln!("  {} use premium", args[0]);
        return Ok(());
    }

    let socket_path = std::env::var("LICENSE_AGENT_SOCKET")
        .unwrap_or_else(|_| "/var/run/license-agent.sock".to_string());
    
    let mut app = ExampleApp::new(&socket_path);

    match args[1].as_str() {
        "load" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: load <license_file>");
            }
            app.load_license(&args[2]).await?;
        }
        "validate" => {
            app.validate_license().await?;
        }
        "use" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: use <feature>");
            }
            app.use_feature(&args[2]).await?;
        }
        _ => {
            anyhow::bail!("Commande inconnue: {}", args[1]);
        }
    }

    Ok(())
}
