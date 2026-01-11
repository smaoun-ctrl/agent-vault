use crate::types::AgentError;
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Gestionnaire cryptographique pour RSA
pub struct CryptoManager {
    private_key: Arc<RsaPrivateKey>,
    public_key: Arc<RsaPublicKey>,
}

impl CryptoManager {
    /// Crée un nouveau gestionnaire avec une paire de clés
    pub fn new(private_key: RsaPrivateKey, public_key: RsaPublicKey) -> Self {
        Self {
            private_key: Arc::new(private_key),
            public_key: Arc::new(public_key),
        }
    }

    /// Charge depuis fichiers PEM
    pub fn from_pem_files(private_key_path: &str, public_key_path: Option<&str>) -> anyhow::Result<Self> {
        use std::fs;

        // Charger clé privée
        let private_key_pem = fs::read_to_string(private_key_path)?;
        let private_key = Self::parse_private_key_pem(&private_key_pem)?;

        // Charger clé publique
        let public_key = if let Some(path) = public_key_path {
            let public_key_pem = fs::read_to_string(path)?;
            Self::parse_public_key_pem(&public_key_pem)?
        } else {
            // Extraire clé publique depuis clé privée
            RsaPublicKey::from(&private_key)
        };

        Ok(Self::new(private_key, public_key))
    }

    /// Génère une nouvelle paire de clés RSA-2048
    pub fn generate_keys() -> anyhow::Result<(RsaPrivateKey, RsaPublicKey)> {
        use rand::rngs::OsRng;
        
        let mut rng = OsRng;
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits)?;
        let public_key = RsaPublicKey::from(&private_key);
        
        Ok((private_key, public_key))
    }

    /// Chiffre avec RSA-OAEP
    pub fn encrypt_oaep(&self, data: &[u8], _label: Option<&[u8]>) -> Result<Vec<u8>, AgentError> {
        use rsa::Oaep;
        use rand::rngs::OsRng;
        
        let padding = Oaep::new::<Sha256>();
        let mut rng = OsRng;
        
        self.public_key
            .encrypt(&mut rng, padding, data)
            .map_err(|e| AgentError::CryptoError(format!("RSA-OAEP encryption failed: {}", e)))
    }

    /// Déchiffre avec RSA-OAEP
    pub fn decrypt_oaep(&self, encrypted: &[u8], _label: Option<&[u8]>) -> Result<Vec<u8>, AgentError> {
        use rsa::Oaep;
        
        let padding = Oaep::new::<Sha256>();
        
        self.private_key
            .decrypt(padding, encrypted)
            .map_err(|e| AgentError::CryptoError(format!("RSA-OAEP decryption failed: {}", e)))
    }

    /// Signe avec RSA-PSS
    pub fn sign_pss(&self, data: &[u8]) -> Result<Vec<u8>, AgentError> {
        use rsa::Pss;
        use rand::rngs::OsRng;
        
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        
        let mut rng = OsRng;
        let padding = Pss::new_with_salt::<Sha256>(&mut rng);
        
        self.private_key
            .sign(padding, &hash)
            .map_err(|e| AgentError::CryptoError(format!("RSA-PSS signing failed: {}", e)))
    }

    /// Vérifie une signature RSA-PSS
    pub fn verify_pss(&self, data: &[u8], signature: &[u8]) -> Result<bool, AgentError> {
        use rsa::Pss;
        use rand::rngs::OsRng;
        
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        
        let mut rng = OsRng;
        let padding = Pss::new_with_salt::<Sha256>(&mut rng);
        
        match self.public_key.verify(padding, &hash, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Obtient la clé publique (pour export)
    pub fn public_key(&self) -> &RsaPublicKey {
        &self.public_key
    }

    /// Exporte la clé publique en PEM
    pub fn export_public_key_pem(&self) -> anyhow::Result<String> {
        use rsa::pkcs1::EncodeRsaPublicKey;
        
        Ok(self.public_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)?)
    }

    /// Exporte la clé privée en PEM
    pub fn export_private_key_pem(&self) -> anyhow::Result<String> {
        use rsa::pkcs1::EncodeRsaPrivateKey;
        
        Ok(self.private_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)?)
    }

    fn parse_private_key_pem(pem: &str) -> anyhow::Result<RsaPrivateKey> {
        use rsa::pkcs1::DecodeRsaPrivateKey;
        
        Ok(RsaPrivateKey::from_pkcs1_pem(pem)?)
    }

    fn parse_public_key_pem(pem: &str) -> anyhow::Result<RsaPublicKey> {
        use rsa::pkcs1::DecodeRsaPublicKey;
        
        Ok(RsaPublicKey::from_pkcs1_pem(pem)?)
    }
}

/// Hash SHA-256
pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Génère un nonce aléatoire
pub fn generate_nonce(size: usize) -> Vec<u8> {
    use rand::Rng;
    let mut nonce = vec![0u8; size];
    rand::thread_rng().fill(&mut nonce[..]);
    nonce
}

/// Comparaison temps constant
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}
