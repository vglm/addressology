use crate::api::user::{ALLOWED_EMAILS, WEB_PORTAL_DOMAIN};
use crate::api::utils::extract_url_param;
use crate::db::ops::{get_and_remove_oauth_stage, get_user};
use crate::oauth::{create_oauth_query, oauth_challenge_and_get_token, verify_access_token};
use crate::ServerData;
use actix_session::Session;
use actix_web::web::Data;
use actix_web::HttpResponse;
use actix_web::{web, HttpRequest, HttpResponseBuilder};
use sqlx::Error;

pub async fn handle_login_via_google(data: web::Data<Box<ServerData>>) -> HttpResponse {
    let db_conn = data.db_connection.lock().await;

    let query = match create_oauth_query(db_conn.clone(), WEB_PORTAL_DOMAIN.clone()).await {
        Ok(query) => query,
        Err(err) => {
            log::error!("Error creating oauth query: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to create oauth query");
        }
    };

    HttpResponseBuilder::new(actix_web::http::StatusCode::TEMPORARY_REDIRECT)
        .append_header((actix_web::http::header::LOCATION, query))
        .finish()
}

pub async fn handle_google_callback(
    data: Data<Box<ServerData>>,
    request: HttpRequest,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let code = extract_url_param(&request, "code")?;
    let state = extract_url_param(&request, "state")?;

    if let (Some(code), Some(state)) = (code, state) {
        // Exchange the code with a token.
        let obj = {
            let conn = data.db_connection.lock().await;
            get_and_remove_oauth_stage(&conn, &state)
                .await
                .map_err(|err: Error| {
                    log::error!("Error getting oauth stage: {:?}", err);
                    actix_web::error::ErrorInternalServerError("Failed to get oauth stage")
                })?
                .ok_or(actix_web::error::ErrorBadRequest(
                    "Failed to get oauth stage",
                ))?
        };

        let token = match oauth_challenge_and_get_token(code, obj.pkce_code_verifier).await {
            Ok(tok) => tok,
            Err(err) => {
                log::error!("Error getting token: {:?}", err);
                return Ok(HttpResponse::InternalServerError().body("Failed to get token"));
            }
        };

        let obj = verify_access_token(token).await.map_err(|err| {
            log::error!("Error verifying token: {:?}", err);
            actix_web::error::ErrorInternalServerError("Failed to verify token")
        })?;

        if let Some(email) = obj.email {
            if !ALLOWED_EMAILS.contains(&email) {
                return Ok(HttpResponse::Unauthorized().body("This email is not allowed"));
            }

            match get_user(&data.db_connection.lock().await.clone(), &email).await {
                Ok(usr) => {
                    if !usr.allow_google_login {
                        log::error!("User {} is not allowed to login with google", email);
                        return Ok(HttpResponse::Unauthorized().body("This email is not allowed"));
                    }
                    session.insert("user", &usr)?;
                    Ok(
                        HttpResponseBuilder::new(actix_web::http::StatusCode::TEMPORARY_REDIRECT)
                            .append_header((actix_web::http::header::LOCATION, "/dashboard/"))
                            .finish(),
                    )
                }
                Err(err) => {
                    log::error!("Error getting user: {}", err);
                    Ok(HttpResponse::InternalServerError()
                        .body(format!("Failed to get user {}", email)))
                }
            }
        } else {
            Ok(HttpResponse::Unauthorized().body("User not found"))
        }
    } else {
        Ok(HttpResponse::BadRequest().body("Missing code or state"))
    }
}
