// Tests d'intégration basiques

#[cfg(test)]
mod tests {
    use license_secret_agent::crypto::CryptoManager;
    use license_secret_agent::types::*;

    #[test]
    fn test_crypto_manager_generation() {
        let (private_key, public_key) = CryptoManager::generate_keys().unwrap();
        let crypto = CryptoManager::new(private_key, public_key);
        
        let data = b"test data";
        let encrypted = crypto.encrypt_oaep(data, None).unwrap();
        let decrypted = crypto.decrypt_oaep(&encrypted, None).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_crypto_sign_verify() {
        let (private_key, public_key) = CryptoManager::generate_keys().unwrap();
        let crypto = CryptoManager::new(private_key, public_key);
        
        let data = b"test data to sign";
        let signature = crypto.sign_pss(data).unwrap();
        let verified = crypto.verify_pss(data, &signature).unwrap();
        
        assert!(verified);
    }

    #[test]
    fn test_constant_time_compare() {
        use license_secret_agent::crypto::constant_time_compare;
        
        assert!(constant_time_compare(b"test", b"test"));
        assert!(!constant_time_compare(b"test", b"test2"));
        assert!(!constant_time_compare(b"test", b""));
    }
}
