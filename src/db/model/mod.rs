mod contract;

pub use contract::*;
use std::collections::BTreeMap;

use crate::types::DbAddress;
use chrono::NaiveDateTime;

use crate::fancy::FancyScoreCategory;
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

    pub tokens: i64,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OauthStageDbObj {
    pub csrf_state: String,
    pub pkce_code_verifier: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct FancyScoreEntry {
    pub category: FancyScoreCategory,
    pub score: f64,
    pub difficulty: f64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FancyScore {
    pub address_mixed_case: String,
    pub address_lower_case: String,
    pub address_short_etherscan: String,
    pub scores: BTreeMap<String, FancyScoreEntry>,
    pub total_score: f64,
    pub price_multiplier: f64,
    pub category: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FancyDbObj {
    pub address: DbAddress,
    pub salt: String,
    pub factory: DbAddress,
    pub created: NaiveDateTime,
    pub score: f64,
    pub owner: Option<String>,
    pub price: i64,
    pub category: String,
    pub job: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContractCreateFromApi {
    pub address: Option<String>,
    pub network: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinerDbObj {
    pub uid: String,
    pub prov_node_id: Option<DbAddress>,
    pub prov_reward_addr: Option<DbAddress>,
    pub prov_name: Option<String>,
    pub prov_extra_info: Option<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobDbObj {
    pub uid: String,
    pub cruncher_ver: String,
    pub started_at: NaiveDateTime,
    pub finished_at: Option<NaiveDateTime>,
    pub requestor_id: Option<DbAddress>,
    pub hashes_reported: f64,
    pub hashes_accepted: f64,
    pub cost_reported: f64,
    pub miner: String,
    pub job_extra_info: Option<String>,
}
