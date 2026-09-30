//! Argon2id password/PIN hashing (OWASP default parameters).

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher as ArgonPasswordHasher, PasswordVerifier,
    SaltString,
};
use argon2::Argon2;
use baraka_application::errors::ApplicationError;
use baraka_application::ports::PasswordHasher;

/// Argon2id hasher with crate-default (OWASP) parameters.
///
/// Passwords and PINs are two independent secret slots in storage; the
/// algorithm is identical for both. Hashes are stored as PHC strings.
#[derive(Debug, Clone, Copy, Default)]
pub struct Argon2PasswordHasher;

impl PasswordHasher for Argon2PasswordHasher {
    fn hash(&self, plaintext: &str) -> Result<String, ApplicationError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(plaintext.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| ApplicationError::Port("password hashing failed".to_string()))
    }

    fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, ApplicationError> {
        let parsed = PasswordHash::new(hash)
            .map_err(|_| ApplicationError::Port("password store unreadable".to_string()))?;
        Ok(Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok())
    }
}
