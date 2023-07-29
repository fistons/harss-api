use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;

#[tracing::instrument(skip(pwd))]
pub fn encode_password(pwd: &str) -> String {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(pwd.as_bytes(), &salt)
        .unwrap()
        .to_string();

    password_hash
}

#[tracing::instrument(skip_all)]
pub fn match_password(user_password: &str, candidate: &str) -> bool {
    let parsed_hash = PasswordHash::new(&user_password).unwrap();
    Argon2::default()
        .verify_password(candidate.as_bytes(), &parsed_hash)
        .is_ok()
}
