use crate::api::user::utils::{check_pass, CheckPassResponse};
use crate::api::user::{pass_to_hash, ALLOWED_EMAILS};
use crate::db::ops::{get_user, update_user_password};
use crate::ServerData;
use actix_session::Session;
use actix_web::web;
use actix_web::web::Data;
use actix_web::HttpResponse;
use rand::Rng;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::time::Duration;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetNewPassData {
    pub email: String,
    pub token: String,
    pub new_password: String,
}

pub(super) async fn set_password_to_response(
    session: Session,
    db_conn: &SqlitePool,
    email: &str,
    new_password_hash: &str,
) -> HttpResponse {
    // Update the user's password in the database
    log::info!("Updating password for user: {}", email);
    match update_user_password(db_conn, email, new_password_hash).await {
        Ok(_) => {
            //logout user
            session.remove("user");
            log::info!(
                "Password successfully updated for user: {}. User logged out",
                email
            );
            HttpResponse::Ok().body("Password changed successfully")
        }
        Err(err) => {
            log::error!("Error updating password for user: {}: {}", email, err);
            HttpResponse::InternalServerError().body("Failed to change password")
        }
    }
}

pub async fn handle_password_set(
    data: Data<Box<ServerData>>,
    change_pass: web::Json<SetNewPassData>,
    session: Session,
) -> HttpResponse {
    let email = change_pass.email.trim().to_lowercase();
    if !ALLOWED_EMAILS.contains(&email) {
        return HttpResponse::Unauthorized().body("This email is not allowed");
    }
    // Simulate a small delay for security reasons (to prevent timing attacks)
    let mut rng = rand::thread_rng();
    let random_duration = rng.gen_range(300..=600);
    tokio::time::sleep(Duration::from_millis(random_duration)).await;

    let db_conn = data.db_connection.lock().await;

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
    log::info!("Checking token for user: {}", email);
    if usr
        .set_pass_token
        .map(|t| t != change_pass.token)
        .unwrap_or(true)
    {
        return HttpResponse::BadRequest().body("Invalid reset token");
    }
    let oldest_possible_token_date = chrono::Utc::now() - chrono::Duration::minutes(10);
    if usr
        .set_pass_token_date
        .map(|t| t < oldest_possible_token_date)
        .unwrap_or(true)
    {
        return HttpResponse::BadRequest().body("Reset token expired");
    }
    //check password strength

    if let CheckPassResponse::BadPassword(resp) = check_pass(&change_pass.new_password) {
        return resp;
    };

    // Hash the new password
    let new_password_hash = pass_to_hash(change_pass.new_password.as_bytes());

    set_password_to_response(session, &db_conn, &email, &new_password_hash).await
}
