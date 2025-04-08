use crate::api::user::pass_to_hash;
use crate::api::user::set_pass::set_password_to_response;
use crate::api::user::utils::{check_pass, CheckPassResponse};
use crate::db::model::UserDbObj;
use crate::db::ops::get_user;
use crate::ServerData;
use actix_session::Session;
use actix_web::web;
use actix_web::web::Data;
use actix_web::HttpResponse;
use rand::Rng;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangePassData {
    pub email: String,
    pub old_password: String,
    pub new_password: String,
}

pub async fn handle_password_change(
    data: Data<Box<ServerData>>,
    change_pass: web::Json<ChangePassData>,
    session: Session,
) -> HttpResponse {
    if session.get::<UserDbObj>("user").unwrap_or(None).is_none() {
        return HttpResponse::Unauthorized().body("Not logged in");
    }
    let email = change_pass.email.trim().to_lowercase();

    // Simulate a small delay for security reasons (to prevent timing attacks)
    let mut rng = rand::rng();
    let random_duration = rng.random_range(300..=600);
    tokio::time::sleep(Duration::from_millis(random_duration)).await;

    let db_conn = data.db_connection.lock().await;

    // Hash the old password
    let old_password_hash = pass_to_hash(change_pass.old_password.as_bytes());

    // Fetch the user from the database using the provided email
    log::info!("Fetching user: {}", email);
    let usr = match get_user(&*db_conn, &email).await {
        Ok(usr) => usr,
        Err(err) => {
            log::error!("Error getting user: {}", err);
            return HttpResponse::Unauthorized().body("Invalid email or password");
        }
    };

    // Check if the provided old password matches the stored password hash
    log::info!("Checking old password hash for user: {}", email);
    if usr.pass_hash != old_password_hash {
        return HttpResponse::Unauthorized().body("Invalid old password");
    }

    //check password strength

    if let CheckPassResponse::BadPassword(resp) = check_pass(&change_pass.new_password) {
        return resp;
    };
    // Hash the new password
    let new_password_hash = pass_to_hash(change_pass.new_password.as_bytes());

    set_password_to_response(session, &db_conn, &email, &new_password_hash).await
}
