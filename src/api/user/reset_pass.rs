use crate::api::user::{ALLOWED_EMAILS, ALLOW_CREATING_NEW_ACCOUNTS, WEB_PORTAL_DOMAIN};
use crate::db::model::UserDbObj;
use crate::db::ops::{get_user, insert_user, save_reset_token};
use crate::email::{send_email, Email};
use crate::ServerData;
use actix_web::web;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder};
use rand::Rng;
use serde::Deserialize;
use url::form_urlencoded;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordData {
    pub email: String,
}

pub async fn handle_password_reset(
    data: Data<Box<ServerData>>,
    reset_data: web::Json<ResetPasswordData>,
) -> impl Responder {
    let email = reset_data.email.trim().to_lowercase();
    if !ALLOWED_EMAILS.contains(&email) {
        return HttpResponse::Unauthorized().body("This email is not allowed");
    }

    let db_conn = data.db_connection.lock().await;

    let user = match get_user(&*db_conn, &email).await {
        Ok(user) => user,
        Err(_err) => {
            if !*ALLOW_CREATING_NEW_ACCOUNTS {
                log::error!("Error getting user or nod found: {}", email);
                return HttpResponse::Unauthorized().body("This email is not allowed");
            }
            // create new user if not found
            let created_date = chrono::Utc::now();
            let user_to_insert = UserDbObj {
                uid: uuid::Uuid::new_v4().to_string(),
                email: email.clone(),
                pass_hash: "000000000000000".to_string(),
                created_date,
                last_pass_change: created_date,
                allow_pass_login: true,
                allow_google_login: false,
                set_pass_token: None,
                set_pass_token_date: None,
                tokens: 0,
            };
            match insert_user(&db_conn, &user_to_insert).await {
                Ok(user) => user,
                Err(err) => {
                    log::error!("Error inserting user: {}", err);
                    return HttpResponse::InternalServerError().body("Failed to insert user");
                }
            }
        }
    };

    let at_least_1_minute_ago = chrono::Utc::now() - chrono::Duration::minutes(1);
    if user
        .set_pass_token_date
        .map(|t| t > at_least_1_minute_ago)
        .unwrap_or(false)
    {
        return HttpResponse::BadRequest().body("Reset token already sent, wait at least a minute");
    }

    let mut rng = rand::thread_rng();
    let mut str = "reset".to_string();
    for _ in 0..16 {
        //gen hex character
        let num = format!("{:x}", rng.gen_range(0..16));
        str.push_str(&num.to_string());
    }

    match save_reset_token(&db_conn, &email, &str).await {
        Ok(()) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to save reset token: {}", err));
        }
    }
    let email_encoded: String = form_urlencoded::byte_serialize(email.as_bytes()).collect();
    send_email(Email {
        recipient: email.clone(),
        subject: "Password reset".to_string(),
        message: format!(
            "Hello, you have requested a password reset. \nPlease click on the following link to reset your password: \nhttps://{}/dashboard/login?reset_token={}&email={}",
            WEB_PORTAL_DOMAIN.as_str(),
            str,
            &email_encoded
        ),
    }).await;
    HttpResponse::Ok().body("Reset link sent")
}
