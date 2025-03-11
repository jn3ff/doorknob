use std::{
    fs,
    io::{Write, stdin, stdout},
};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    password_hash
}

fn save_password(password: &str) {
    let hash = hash_password(password);

    fs::write("password_hash.txt", hash).expect("password hash needs to be saved correctly");

    println!("Password hash saved successfully.");
}

pub fn setup_password() {
    println!("Please set the doorlock password");
    let _ = stdout().flush();
    let mut s: String = String::new();
    stdin().read_line(&mut s).unwrap();

    s = s.trim().to_string();
    if s.len() < 1 {
        panic!("No password set")
    }

    save_password(s.as_str());
}

pub fn verify_password(checkpass: &str) -> Result<bool, anyhow::Error> {
    let hashed_sot = fs::read_to_string("password_hash.txt")?;
    let parsed_hash = PasswordHash::new(&hashed_sot)?;

    let argon2 = Argon2::default();
    let result = argon2
        .verify_password(checkpass.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(result)
}
