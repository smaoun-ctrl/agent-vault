use crate::license::LicenseValidator;
use crate::types::{ValidateLicenseRequest, ValidateLicenseResponse};
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, warn};

/// Serveur IPC (Unix Domain Socket)
pub struct IpcServer {
    listener: UnixListener,
    validator: Arc<LicenseValidator>,
    allowed_uids: Vec<u32>,
}

impl IpcServer {
    pub async fn new<P: AsRef<Path>>(
        socket_path: P,
        validator: Arc<LicenseValidator>,
        allowed_uids: Vec<u32>,
    ) -> anyhow::Result<Self> {
        // Supprimer socket existant si présent
        if socket_path.as_ref().exists() {
            std::fs::remove_file(&socket_path)?;
        }

        // Créer répertoire parent si nécessaire
        if let Some(parent) = socket_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Créer listener
        let listener = UnixListener::bind(&socket_path)?;
        
        // Permissions socket (600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&socket_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&socket_path, perms)?;
        }

        info!("IPC server listening on {}", socket_path.as_ref().display());

        Ok(Self {
            listener,
            validator,
            allowed_uids,
        })
    }

    /// Démarre le serveur IPC
    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New IPC connection from {:?}", addr);
                    
                    let validator = Arc::clone(&self.validator);
                    let allowed_uids = self.allowed_uids.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, validator, allowed_uids).await {
                            error!("Error handling IPC connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept IPC connection: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn handle_connection(
        mut stream: UnixStream,
        validator: Arc<LicenseValidator>,
        allowed_uids: Vec<u32>,
    ) -> anyhow::Result<()> {
        // Vérifier UID du client
        let peer_uid = Self::get_peer_uid(&stream)?;
        
        if !allowed_uids.is_empty() && !allowed_uids.contains(&peer_uid) {
            warn!("Rejected connection from unauthorized UID: {}", peer_uid);
            return Err(anyhow::anyhow!("Unauthorized UID: {}", peer_uid));
        }

        debug!("Accepted connection from UID: {}", peer_uid);

        // Lire requête
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        // Lire longueur (4 bytes)
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > 1024 * 1024 {
            return Err(anyhow::anyhow!("Request too large: {} bytes", len));
        }

        // Lire données
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;

        // Désérialiser requête
        let request: ValidateLicenseRequest = serde_json::from_slice(&data)
            .map_err(|e| anyhow::anyhow!("Failed to parse request: {}", e))?;

        // Valider licence
        let result = validator.validate(&request.license_token).await;

        let response = match result {
            Ok(validation_result) => ValidateLicenseResponse {
                result: validation_result,
            },
            Err(e) => ValidateLicenseResponse {
                result: crate::types::ValidationResult {
                    valid: false,
                    expires_at: None,
                    features: vec![],
                    metadata: std::collections::HashMap::new(),
                    error: Some(e.to_string()),
                },
            },
        };

        // Sérialiser réponse
        let response_json = serde_json::to_vec(&response)?;
        let response_len = response_json.len() as u32;

        // Envoyer longueur + données
        stream.write_all(&response_len.to_be_bytes()).await?;
        stream.write_all(&response_json).await?;
        stream.flush().await?;

        debug!("Response sent to UID: {}", peer_uid);

        Ok(())
    }

    fn get_peer_uid(stream: &UnixStream) -> anyhow::Result<u32> {
        use nix::sys::socket::{getsockopt, sockopt};
        
        let creds = getsockopt(stream, sockopt::PeerCredentials)
            .map_err(|e| anyhow::anyhow!("Failed to get peer credentials: {}", e))?;
        Ok(creds.uid() as u32)
    }
}
