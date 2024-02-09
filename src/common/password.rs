use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;
use secrecy::{ExposeSecret, Secret};

/// Encode the password using argon2
#[tracing::instrument(skip(password))]
pub fn encode_password(password: &Secret<String>) -> String {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(password.expose_secret().as_bytes(), &salt)
        .unwrap()
        .to_string();

    password_hash
}

/// Check if the candidate match the hashed user password
#[tracing::instrument(skip_all)]
pub fn verify_password(user_password: &str, candidate: &Secret<String>) -> bool {
    let parsed_hash = PasswordHash::new(user_password).unwrap();
    Argon2::default()
        .verify_password(candidate.expose_secret().as_bytes(), &parsed_hash)
        .is_ok()
}
