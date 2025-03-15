use std::{
    fs,
    io::{Write, stdin, stdout},
};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

static PASSWORD_HASH: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new(String::new()));

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    password_hash
}

async fn cache_hash(password_hash: String) {
    let mut cacheword = PASSWORD_HASH.write().await;
    *cacheword = password_hash;
}

async fn save_password(password: &str) {
    let hash = hash_password(password);

    fs::write("password_hash.txt", hash.clone())
        .expect("password hash needs to be saved correctly");

    cache_hash(hash).await;

    println!("Password hash saved successfully.");
}

pub async fn setup_password() {
    println!("Please set the doorlock password");
    let _ = stdout().flush();
    let mut s: String = String::new();
    stdin().read_line(&mut s).unwrap();

    s = s.trim().to_string();
    if s.is_empty() {
        panic!("No password set")
    }

    save_password(s.as_str()).await;
}

pub async fn verify_password(checkpass: &str) -> Result<bool, anyhow::Error> {
    let hash = {
        let cached_hash = PASSWORD_HASH.read().await.clone();
        if cached_hash.is_empty() {
            println!("password cache is empty. Justin you fucked up the control flow");
            let hash_sot = fs::read_to_string("password_hash.txt")?;
            cache_hash(hash_sot.clone()).await;
            hash_sot
        } else {
            cached_hash
        }
    };
    let parsed_hash = PasswordHash::new(&hash)?;

    let argon2 = Argon2::default();
    let result = argon2
        .verify_password(checkpass.as_bytes(), &parsed_hash)
        .is_ok();

    Ok(result)
}
