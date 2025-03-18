// Code adapted from https://github.com/ramosbugs/oauth2-rs/blob/main/examples/google.rs
//
// Must set the enviroment variables:
// GOOGLE_CLIENT_ID=xxx
// GOOGLE_CLIENT_SECRET=yyy

use crate::api::user::WEB_PORTAL_DOMAIN;
use crate::db::model::OauthStageDbObj;
use crate::db::ops::{delete_old_oauth_stages, insert_oauth_stage};
use crate::db::utils::get_current_utc_time;
use crate::err_custom_create;
use crate::error::AddressologyError;
use dotenvy::var;
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RequestTokenError, RevocationUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::Deserialize;
use sqlx::PgPool;

fn get_client(hostname: String) -> Result<BasicClient, AddressologyError> {
    let google_client_id = ClientId::new(var("GOOGLE_CLIENT_ID").unwrap());
    let google_client_secret = ClientSecret::new(var("GOOGLE_CLIENT_SECRET").unwrap());
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string()).unwrap();
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap();

    let protocol = if hostname.starts_with("localhost") || hostname.starts_with("127.0.0.1") {
        "http"
    } else {
        "https"
    };

    let redirect_url = format!("{}://{}/api/auth/callback/google", protocol, hostname);

    // Set up the config for the Google OAuth2 process.
    let client = BasicClient::new(
        google_client_id,
        Some(google_client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(
        RedirectUrl::new(redirect_url)
            .map_err(|_| err_custom_create!("OAuth: invalid redirect URL"))?,
    )
    .set_revocation_uri(
        RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string())
            .map_err(|_| err_custom_create!("OAuth: invalid revocation endpoint URL"))?,
    );
    Ok(client)
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthTokenInfo {
    pub expires_in: Option<i64>,
    pub email: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub async fn verify_access_token(
    access_token: String,
) -> Result<OAuthTokenInfo, AddressologyError> {
    let req = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v1/tokeninfo")
        .query(&[("access_token", access_token)])
        .send()
        .await
        .map_err(|err| {
            log::error!("OAuth: failed to send request: {:?}", err);
            err_custom_create!("OAuth: failed to send request")
        })?;

    let payload = req.text().await.map_err(|err| {
        log::error!("OAuth: failed to get response text: {:?}", err);
        err_custom_create!("OAuth: failed to get response text")
    })?;
    let info = serde_json::from_str::<OAuthTokenInfo>(&payload).map_err(|err| {
        log::error!("OAuth: failed to parse response: {:?}", err);
        err_custom_create!("OAuth: failed to parse response")
    })?;

    Ok(info)
}

pub async fn create_oauth_query(
    db_conn: PgPool,
    hostname: String,
) -> Result<String, AddressologyError> {
    let (pkce_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    // Exchange the code with a token.
    let client = get_client(hostname)?;

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        // Set the desired scopes.
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/userinfo.email".to_string(),
        ))
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();

    delete_old_oauth_stages(&db_conn).await.map_err(|err| {
        log::error!("OAuth: failed to delete old oauth stages: {:?}", err);
        err_custom_create!("OAuth: failed to delete old oauth stages")
    })?;
    insert_oauth_stage(
        &db_conn,
        OauthStageDbObj {
            csrf_state: csrf_token.secret().to_string(),
            pkce_code_verifier: pkce_code_verifier.secret().to_string(),
            created_at: get_current_utc_time(),
        },
    )
    .await
    .map_err(|err| {
        log::error!("OAuth: failed to insert oauth stage: {:?}", err);
        err_custom_create!("OAuth: failed to insert oauth stage")
    })?;
    Ok(auth_url.to_string())
}

pub async fn oauth_challenge_and_get_token(
    code: String,
    verifier: String,
) -> Result<String, AddressologyError> {
    // Exchange the code with a token.
    let client = get_client(WEB_PORTAL_DOMAIN.clone())?;

    let code = AuthorizationCode::new(code);

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(PkceCodeVerifier::new(verifier))
        .request_async(async_http_client)
        .await
        .map_err(|err| {
            match err {
                RequestTokenError::ServerResponse(err) => {
                    log::error!("OAuth: Server response error: {:?}", err);
                }
                RequestTokenError::Request(err) => {
                    log::error!("OAuth: Request error: {:?}", err);
                }
                RequestTokenError::Parse(err, data) => {
                    log::error!("OAuth: Parse error: {:?}, {:?}", err, data);
                }
                RequestTokenError::Other(other) => {
                    log::error!("OAuth: Other error: {:?}", other);
                }
            };
            err_custom_create!("OAuth: failed to exchange code for token")
        })?;

    Ok(token_response.access_token().secret().into())
}
