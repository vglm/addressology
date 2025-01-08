use crate::types::DbAddress;
use chrono::NaiveDateTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDbObj {
    pub uid: String,
    pub email: String,
    #[serde(skip)]
    pub pass_hash: String,
    pub created_date: DateTime<Utc>,
    pub last_pass_change: DateTime<Utc>,

    #[serde(skip)]
    pub set_pass_token: Option<String>,
    #[serde(skip)]
    pub set_pass_token_date: Option<DateTime<Utc>>,

    pub allow_pass_login: bool,
    pub allow_google_login: bool,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OauthStageDbObj {
    pub csrf_state: String,
    pub pkce_code_verifier: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FancyDbObj {
    pub address: DbAddress,
    pub salt: String,
    pub factory: DbAddress,
    pub created: NaiveDateTime,
    pub score: f64,
    pub miner: String,
}
