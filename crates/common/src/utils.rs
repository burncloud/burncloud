use crate::error::{BurnCloudError, Result};
use std::fs;
use std::path::Path;

pub fn ensure_dir_exists(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn get_home_dir() -> String {
    dirs::home_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .to_string_lossy()
        .to_string()
}

/// Hash a password using bcrypt with default cost (12)
pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| BurnCloudError::PasswordHashError(e.to_string()))
}

/// Verify a password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    bcrypt::verify(password, hash).map_err(|e| BurnCloudError::PasswordHashError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Failed to hash password");

        // Bcrypt hashes should start with $2 (bcrypt identifier)
        assert!(hash.starts_with("$2"));
        // Bcrypt hashes are typically 60 characters
        assert_eq!(hash.len(), 60);
    }

    #[test]
    fn test_verify_password_success() {
        let password = "correct_password";
        let hash = hash_password(password).expect("Failed to hash password");

        let result = verify_password(password, &hash).expect("Failed to verify password");
        assert!(result, "Password verification should succeed");
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).expect("Failed to hash password");

        let result = verify_password(wrong_password, &hash).expect("Failed to verify password");
        assert!(
            !result,
            "Password verification should fail for wrong password"
        );
    }

    #[test]
    fn test_hash_same_password_different_hashes() {
        let password = "same_password";
        let hash1 = hash_password(password).expect("Failed to hash password");
        let hash2 = hash_password(password).expect("Failed to hash password");

        // Same password should produce different hashes due to random salt
        assert_ne!(hash1, hash2);

        // But both should verify successfully
        assert!(verify_password(password, &hash1).expect("Failed to verify"));
        assert!(verify_password(password, &hash2).expect("Failed to verify"));
    }
}
