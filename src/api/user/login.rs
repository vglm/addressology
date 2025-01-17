use crate::api::user::{pass_to_hash, ALLOWED_EMAILS};
use crate::db::model::UserDbObj;
use crate::db::ops::get_user;
use crate::ServerData;
use actix_session::Session;
use actix_web::web;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder};
use rand::Rng;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub email: String,
    pub password: String,
}

pub async fn handle_login(
    data: Data<Box<ServerData>>,
    login: web::Json<LoginData>,
    session: Session,
) -> impl Responder {
    let email = login.email.trim().to_lowercase();
    if let Some(user) = session.get::<UserDbObj>("user").unwrap_or(None) {
        return if email == user.email {
            HttpResponse::Ok().json(user)
        } else {
            HttpResponse::BadRequest().body("Already logged in as a different user")
        };
    }

    // Generate a random number between 300 and 500 (in milliseconds)
    let mut rng = rand::thread_rng();
    let random_duration = rng.gen_range(300..=600);
    tokio::time::sleep(Duration::from_millis(random_duration)).await;

    if !ALLOWED_EMAILS.contains(&email) {
        return HttpResponse::Unauthorized().body("This email is not allowed");
    }

    let db_conn = data.db_connection.lock().await;

    let key = pass_to_hash(login.password.as_bytes());

    //log::info!("Getting user: {}", email);
    let usr = match get_user(&*db_conn, &email).await {
        Ok(usr) => usr,
        Err(err) => {
            log::error!("Error getting user: {}", err);
            return HttpResponse::Unauthorized().body("Invalid email or password");
        }
    };
    if !usr.allow_pass_login {
        log::error!("User {} is not allowed to login with password", email);
        return HttpResponse::Unauthorized().body("Invalid email or password");
    }
    //log::info!("Login {} == {}", usr.pass_hash, key);
    if usr.pass_hash == key {
        log::info!("User {} logged in", email);
        session.insert("user", &usr).unwrap();

        return HttpResponse::Ok().json(usr);
    }
    HttpResponse::Unauthorized().body("Invalid email or password")
}
