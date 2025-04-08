use crate::db::ops::{
    fancy_get_job_info, fancy_update_job, get_or_insert_factory, get_or_insert_public_key,
    insert_fancy_obj,
};
use crate::fancy::{parse_fancy, parse_fancy_private};
use crate::types::DbAddress;
use crate::ServerData;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewData {
    pub salt: String,
    pub factory: String,
    pub address: Option<String>,
    pub job_id: Option<Uuid>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewDataEntry {
    pub salt: String,
    pub factory: String,
    pub address: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReportedExtraInfo {
    pub job_id: Uuid,
    pub reported_hashes: f64,
    pub reported_cost: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewDataMany {
    pub data: Vec<AddNewDataEntry>,
    pub extra: ReportedExtraInfo,
}

pub enum FancyNewResult {
    Ok(HttpResponse),
    ParseError(String),
    Duplicate,
    Error(HttpResponse),
    ScoreTooLow,
}

async fn _handle_fancy_new_with_trans(
    new_data: web::Json<AddNewData>,
    total_score: &mut f64,
    db_trans: &mut Transaction<'_, Postgres>,
) -> FancyNewResult {
    let mut result = if new_data.factory.len() == 42 || new_data.factory.len() == 40 {
        let factory = match web3::types::Address::from_str(&new_data.factory) {
            Ok(factory) => factory,
            Err(e) => {
                log::error!("{}", e);
                return FancyNewResult::Error(HttpResponse::BadRequest().finish());
            }
        };
        let fancy = match parse_fancy(new_data.salt.clone(), factory) {
            Ok(fancy) => fancy,
            Err(e) => {
                log::error!("{}", e);
                return FancyNewResult::Error(HttpResponse::InternalServerError().finish());
            }
        };
        match get_or_insert_factory(db_trans, DbAddress::from_h160(factory)).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("{}", e);
                return FancyNewResult::Error(HttpResponse::InternalServerError().finish());
            }
        }
        fancy
    } else {
        //normalize public key
        let public_key_base = new_data.factory.clone();
        let public_key_bytes = match hex::decode(public_key_base.replace("0x", "")) {
            Ok(bytes) => bytes,
            Err(e) => {
                return FancyNewResult::ParseError(format!("Invalid public key {e}"));
            }
        };
        if public_key_bytes.len() != 64 {
            return FancyNewResult::ParseError(format!(
                "Invalid public key length, should be 64: {}",
                public_key_base.len()
            ));
        }
        let public_key_base = "0x".to_string() + &hex::encode(public_key_bytes);
        let fancy = match parse_fancy_private(public_key_base, new_data.salt.clone()) {
            Ok(fancy) => fancy,
            Err(e) => {
                return FancyNewResult::ParseError(format!("{}", e));
            }
        };
        let public_key_base = match fancy.public_key_base.clone() {
            Some(key) => key,
            None => {
                return FancyNewResult::ParseError("Public key not found after parse".to_string());
            }
        };
        match get_or_insert_public_key(db_trans, &public_key_base).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("{}", e);
                return FancyNewResult::Error(HttpResponse::InternalServerError().finish());
            }
        }
        fancy
    };

    result.job_id = new_data.job_id;

    if result.score < 1E10 {
        log::debug!("Score too low: {}", result.score);
        return FancyNewResult::ScoreTooLow;
    }

    let gen_address = format!("{:#x}", result.address.addr());
    if let Some(reference_address) = new_data.address.as_ref().map(|a| a.to_lowercase()) {
        if gen_address != reference_address {
            return FancyNewResult::ParseError(format!(
                "Address mismatch expected: {}, got: {}",
                gen_address,
                reference_address.to_lowercase()
            ));
        }
    }
    let score = result.score;

    match insert_fancy_obj(&mut **db_trans, result).await {
        Ok(_) => {
            *total_score += score;
            FancyNewResult::Ok(HttpResponse::Ok().json(json!({
                "totalSore": score
            })))
        }
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                FancyNewResult::Duplicate
            } else {
                log::error!("{}", e);
                FancyNewResult::Error(HttpResponse::InternalServerError().finish())
            }
        }
    }
}

pub async fn handle_fancy_new_many(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<AddNewDataMany>,
) -> HttpResponse {
    let mut total_score = 0.0;
    let conn = server_data.db_connection.lock().await;
    let mut db_trans = match conn.begin().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let find_job = match fancy_get_job_info(&mut *db_trans, new_data.extra.job_id).await {
        Ok(job) => job,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut entries_accepted = 0;
    let mut entries_rejected = 0;
    let mut entries_parse_error = 0;
    for data in new_data.data.iter() {
        let new_data = AddNewData {
            salt: data.salt.clone(),
            factory: data.factory.clone(),
            address: data.address.clone(),
            job_id: Some(new_data.extra.job_id),
        };
        let resp =
            _handle_fancy_new_with_trans(web::Json(new_data), &mut total_score, &mut db_trans)
                .await;
        match resp {
            FancyNewResult::Ok(_ok) => {
                entries_accepted += 1;
            }
            FancyNewResult::Duplicate => {
                entries_rejected += 1;
            }
            FancyNewResult::ParseError(err) => {
                //this log can become performance issue, probably should rate limit it
                log::warn!("Error parsing fancy: {}", err);
                entries_rejected += 1;
                entries_parse_error += 1;
            }
            FancyNewResult::Error(err) => {
                return err;
            }
            FancyNewResult::ScoreTooLow => {
                entries_rejected += 1;
                log::debug!("Score too low - skipping");
            }
        }
    }

    match fancy_update_job(
        &mut *db_trans,
        find_job.uid,
        find_job.hashes_accepted + total_score,
        new_data.extra.reported_hashes,
        entries_accepted,
        entries_rejected,
        new_data.extra.reported_cost,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    match db_trans.commit().await {
        Ok(_) => {}
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok().json(json!({
        "totalScore": total_score,
        "entriesRejected": entries_rejected,
        "entriesParseError": entries_parse_error,
    }))
}
