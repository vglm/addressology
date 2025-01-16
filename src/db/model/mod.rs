use std::fmt::Display;
use std::str::FromStr;
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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum DeployStatus {
    None,
    Requested,
    TxSent,
    Failed,
    Succeeded,
}

impl FromStr for DeployStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "requested" => Ok(DeployStatus::Requested),
            "tx_sent" => Ok(DeployStatus::TxSent),
            "failed" => Ok(DeployStatus::Failed),
            "succeeded" => Ok(DeployStatus::Succeeded),
            "" => Ok(DeployStatus::None),
            _ => Err(format!("Invalid deploy status: {}", s)),
        }
    }
}


impl Display for DeployStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeployStatus::None => write!(f, ""),
            DeployStatus::Requested => write!(f, "requested"),
            DeployStatus::TxSent => write!(f, "tx_sent"),
            DeployStatus::Failed => write!(f, "failed"),
            DeployStatus::Succeeded => write!(f, "succeeded"),
        }
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContractDbObj {
    pub contract_id: String,
    pub user_id: String,
    pub created: NaiveDateTime,
    pub address: Option<String>,
    pub network: String,
    pub data: String,
    pub tx: Option<String>,
    pub deploy_status: DeployStatus,
    pub deploy_requested: Option<NaiveDateTime>,
    pub deploy_sent: Option<NaiveDateTime>,
    pub deployed: Option<NaiveDateTime>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContractCreateFromApi {
    pub address: Option<String>,
    pub network: String,
    pub data: String,
}
