mod change_pass;
mod is_login;
mod login;
mod logout;
mod reset_pass;
mod set_pass;
mod utils;

use actix_session::Session;
use actix_web::{HttpResponse, Responder};
use clap::crate_version;
use lazy_static::lazy_static;
use pbkdf2::pbkdf2_hmac_array;
use rustc_hex::ToHex;
use serde_json::json;
use sha2::Sha256;
use std::env;

pub use change_pass::*;
pub use is_login::*;
pub use login::*;
pub use logout::*;
pub use reset_pass::*;
pub use set_pass::*;

#[allow(dead_code)] // referenced lazily; unused under some feature sets
fn get_allowed_emails() -> Vec<String> {
    let res = env::var("ALLOWED_EMAILS")
        .unwrap_or("sieciech.czajka@golem.network".to_string())
        .split(",")
        .map(|x| x.trim().to_string())
        .collect();
    log::info!("Allowed emails loaded: {:?}", res);
    res
}

#[allow(dead_code)] // referenced lazily; unused under some feature sets
fn get_domain() -> String {
    let res = env::var("WEB_PORTAL_DOMAIN").unwrap_or("localhost".to_string());

    log::info!("Portal domain: {}", res);

    res
}

lazy_static! {
    pub static ref ALLOWED_EMAILS: Vec<String> = get_allowed_emails();
    pub static ref WEB_PORTAL_DOMAIN: String = get_domain();
    static ref PASS_SALT: String = env::var("PASS_SALT").unwrap_or("LykwVQJAcU".to_string());
    pub static ref ALLOW_CREATING_NEW_ACCOUNTS: bool = env::var("ALLOW_CREATING_NEW_ACCOUNTS")
        .map(|v| v == "true")
        .unwrap_or(false);
}

fn pass_to_hash(password_binary: &[u8]) -> String {
    //decode password
    let salt = PASS_SALT.as_bytes();
    // number of iterations
    let n = 5000;

    let key: String = pbkdf2_hmac_array::<Sha256, 20>(password_binary, salt, n).to_hex();

    if key.len() != 40 {
        panic!("Key length should be 40")
    }
    key
}

pub async fn handle_session_check(session: Session) -> impl Responder {
    if session.get::<String>("check").unwrap_or(None).is_none() {
        session
            .insert("check", uuid::Uuid::new_v4().to_string())
            .unwrap();
    }
    session.get::<String>("check").unwrap().unwrap()
}

pub async fn handle_greet(session: Session) -> impl Responder {
    println!("Session: {:?}", session.status());
    let describe_version = crate_version!();

    HttpResponse::Ok().json(json!({
        "message": "Hello, World!",
        "domain": *WEB_PORTAL_DOMAIN.clone(),
        "version": describe_version,
    }))
}
