use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::{Database, Decode, Encode, Sqlite};
use std::fmt::Display;
use std::str::FromStr;

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

impl sqlx::Type<sqlx::Sqlite> for DeployStatus {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }
    fn compatible(ty: &<Sqlite as sqlx::Database>::TypeInfo) -> bool {
        <String as sqlx::Type<Sqlite>>::compatible(ty)
    }
}

impl<'r, DB: Database> Decode<'r, DB> for DeployStatus
where
    &'r str: Decode<'r, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'r>) -> sqlx::Result<Self, BoxDynError> {
        let value: &str = Decode::decode(value)?;
        DeployStatus::from_str(value).map_err(Into::into)
    }
}

impl<'q, DB: Database> Encode<'q, DB> for DeployStatus
where
    String: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(&self, buf: &mut DB::ArgumentBuffer<'q>) -> sqlx::Result<IsNull, BoxDynError> {
        Encode::<DB>::encode(self.to_string(), buf)
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
