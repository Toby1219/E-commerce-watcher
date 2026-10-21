use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::SaltString,
    
};
use rand_core::OsRng;

pub fn hash_paswrd(pw: String) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

pub fn verify_hashpaswrd(pw: String, hash: String) -> bool {
    let parsed = argon2::PasswordHash::new(&hash).unwrap();
    Argon2::default().verify_password(pw.as_bytes(), &parsed).is_ok()
}