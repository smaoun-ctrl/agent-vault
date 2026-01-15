use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use clap::Parser;
use license_secret_agent::crypto::{self, CryptoManager};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "license-server")]
#[command(about = "Serveur HTTP pour tester l'agent (rotation + licences)")]
struct Args {
    #[arg(long, default_value = "8080")]
    port: u16,

    #[arg(long, default_value = "server_private_key.pem")]
    private_key_path: String,

    #[arg(long, default_value = "server_public_key.pem")]
    public_key_path: String,

    #[arg(long, default_value = "30")]
    license_duration_days: u64,

    #[arg(long, default_value = "1")]
    secret_version: u64,

    #[arg(long, default_value = "90")]
    secret_validity_days: u64,

    #[arg(long, default_value = "7")]
    grace_period_days: u64,

    #[arg(long)]
    agent_public_key_path: Option<String>,

    /// Activer HTTPS si cert/key fournis (format PEM)
    #[arg(long)]
    tls_cert_path: Option<String>,

    #[arg(long)]
    tls_key_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LicenseRequest {
    customer_id: String,
    license_id: Option<String>,
    features: Vec<String>,
    metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LicenseResponse {
    license_token: String,
    expires_at: DateTime<Utc>,
    license_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RotateSecretRequest {
    agent_id: String,
    current_version: u64,
    timestamp: DateTime<Utc>,
    nonce: String,
    signature: String,
    agent_public_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RotateSecretResponse {
    new_secret_encrypted: String,
    version: u64,
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    grace_until: DateTime<Utc>,
    signature: String,
}

struct LicenseServer {
    crypto: Arc<CryptoManager>,
    current_secret: Vec<u8>,
    secret_version: u64,
    license_duration_days: u64,
    secret_validity_days: u64,
    grace_period_days: u64,
}

impl LicenseServer {
    fn new(
        crypto: Arc<CryptoManager>,
        secret_version: u64,
        license_duration_days: u64,
        secret_validity_days: u64,
        grace_period_days: u64,
    ) -> Result<Self> {
        let current_secret = crypto::generate_nonce(32);

        Ok(Self {
            crypto,
            current_secret,
            secret_version,
            license_duration_days,
            secret_validity_days,
            grace_period_days,
        })
    }

    fn generate_license(
        &self,
        customer_id: String,
        license_id: Option<String>,
        features: Vec<String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<LicenseResponse> {
        use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
        use base64::{engine::general_purpose, Engine as _};

        let license_id = license_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let license_data = serde_json::json!({
            "license_id": license_id,
            "customer_id": customer_id,
            "features": features,
            "metadata": metadata.unwrap_or_default(),
            "expires_at": (Utc::now() + chrono::Duration::days(self.license_duration_days as i64)).to_rfc3339(),
            "issued_at": Utc::now().to_rfc3339(),
        });

        let license_json = serde_json::to_vec(&license_data)?;

        let key = Key::<Aes256Gcm>::from_slice(&self.current_secret);
        let cipher = Aes256Gcm::new(key);

        let nonce_bytes = crypto::generate_nonce(12);
        let nonce = Nonce::from_slice(&nonce_bytes);

        use aead::Aead;
        let ciphertext = cipher
            .encrypt(nonce, license_json.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let mut token = Vec::new();
        token.extend_from_slice(&self.secret_version.to_be_bytes());
        token.extend_from_slice(&nonce_bytes);
        token.extend_from_slice(&ciphertext);

        let license_token = general_purpose::STANDARD.encode(&token);
        let expires_at = Utc::now() + chrono::Duration::days(self.license_duration_days as i64);

        Ok(LicenseResponse {
            license_token,
            expires_at,
            license_id,
        })
    }

    fn generate_new_secret(&mut self) -> Vec<u8> {
        self.secret_version += 1;
        self.current_secret = crypto::generate_nonce(32);
        self.current_secret.clone()
    }

    fn encrypt_secret_for_agent(&self, secret: &[u8], agent_public_key_pem: &str) -> Result<String> {
        use base64::{engine::general_purpose, Engine as _};
        use rsa::pkcs1::DecodeRsaPublicKey;
        use sha2::Sha256;

        let agent_pub_key = RsaPublicKey::from_pkcs1_pem(agent_public_key_pem)?;
        let mut rng = rand::thread_rng();
        let padding = Oaep::new::<Sha256>();
        let encrypted = agent_pub_key.encrypt(&mut rng, padding, secret)?;

        Ok(general_purpose::STANDARD.encode(&encrypted))
    }

    fn handle_rotation(&mut self, agent_public_key_pem: &str) -> Result<RotateSecretResponse> {
        let new_secret = self.generate_new_secret();
        let encrypted_secret = self.encrypt_secret_for_agent(&new_secret, agent_public_key_pem)?;

        let valid_from = Utc::now();
        let valid_until = valid_from + chrono::Duration::days(self.secret_validity_days as i64);
        let grace_until = valid_until + chrono::Duration::days(self.grace_period_days as i64);

        let response_data = serde_json::json!({
            "new_secret_encrypted": encrypted_secret,
            "version": self.secret_version,
            "valid_from": valid_from.to_rfc3339(),
            "valid_until": valid_until.to_rfc3339(),
            "grace_until": grace_until.to_rfc3339(),
        });

        let response_json = serde_json::to_vec(&response_data)?;
        let signature = self.crypto.sign_pss(&response_json)?;
        use base64::{engine::general_purpose, Engine as _};
        let signature_b64 = general_purpose::STANDARD.encode(&signature);

        Ok(RotateSecretResponse {
            new_secret_encrypted: encrypted_secret,
            version: self.secret_version,
            valid_from,
            valid_until,
            grace_until,
            signature: signature_b64,
        })
    }
}

#[derive(Clone)]
struct AppState {
    server: Arc<Mutex<LicenseServer>>,
    agent_public_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("license_server=info")
        .init();

    let args = Args::parse();

    let crypto = if std::path::Path::new(&args.private_key_path).exists() {
        CryptoManager::from_pem_files(&args.private_key_path, Some(&args.public_key_path))?
    } else {
        let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), 2048)?;
        let public_key = private_key.to_public_key();

        use rsa::pkcs1::EncodeRsaPrivateKey;
        use rsa::pkcs1::EncodeRsaPublicKey;
        std::fs::write(&args.private_key_path, private_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)?)?;
        std::fs::write(&args.public_key_path, public_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)?)?;

        CryptoManager::new(private_key, public_key)
    };

    let agent_public_key = if let Some(path) = args.agent_public_key_path.as_ref() {
        Some(std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?)
    } else {
        None
    };

    let crypto = Arc::new(crypto);
    let server = LicenseServer::new(
        Arc::clone(&crypto),
        args.secret_version,
        args.license_duration_days,
        args.secret_validity_days,
        args.grace_period_days,
    )?;

    let state = AppState {
        server: Arc::new(Mutex::new(server)),
        agent_public_key,
    };

    let app = Router::new()
        .route("/api/v1/rotate-secret", post(rotate_secret))
        .route("/api/v1/generate-license", post(generate_license))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);

    if let (Some(cert_path), Some(key_path)) = (args.tls_cert_path.as_ref(), args.tls_key_path.as_ref()) {
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .with_context(|| format!("Failed to load TLS cert/key: {}, {}", cert_path, key_path))?;
        tracing::info!("Serveur HTTPS démarré sur https://{}", addr);
        axum_server::bind_rustls(addr.parse()?, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        tracing::info!("Serveur HTTP démarré sur http://{}", addr);
        axum::serve(tokio::net::TcpListener::bind(&addr).await?, app).await?;
    }
    Ok(())
}

async fn rotate_secret(
    State(state): State<AppState>,
    Json(request): Json<RotateSecretRequest>,
) -> Result<Json<RotateSecretResponse>, (StatusCode, String)> {
    let agent_public_key = request
        .agent_public_key
        .or_else(|| state.agent_public_key.clone())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "agent_public_key manquant".to_string()))?;

    let mut server = state.server.lock().await;
    let response = server
        .handle_rotation(&agent_public_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(response))
}

async fn generate_license(
    State(state): State<AppState>,
    Json(request): Json<LicenseRequest>,
) -> Result<Json<LicenseResponse>, (StatusCode, String)> {
    let server = state.server.lock().await;
    let response = server
        .generate_license(
            request.customer_id,
            request.license_id,
            request.features,
            request.metadata,
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(response))
}
