pub mod score;
pub mod tokens;

use crate::api::contract::api::login_check_fn;
use crate::api::utils::{
    extract_url_bool_param, extract_url_date_param, extract_url_int_param, extract_url_param,
};
use crate::db::model::{ContractAddressDbObj, DeployStatus, JobDbObj, MinerDbObj, UserDbObj};
use crate::db::ops::{
    fancy_finish_job, fancy_get_by_address, fancy_get_job_info, fancy_get_miner_info,
    fancy_insert_job_info, fancy_insert_miner_info, fancy_list, fancy_update_job,
    fancy_update_owner, get_contract_address_list, get_contract_by_id, get_or_insert_factory,
    get_or_insert_public_key, get_public_key_list, get_user, insert_fancy_obj,
    update_contract_data, update_user_tokens, FancyOrderBy, PublicKeyFilter, ReservedStatus,
};
use crate::fancy::{parse_fancy, parse_fancy_private};
use crate::types::DbAddress;
use crate::{get_logged_user_or_null, login_check_and_get, normalize_address, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use pbkdf2::password_hash::rand_core::RngCore;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Sqlite, Transaction};
use std::cmp::PartialEq;
use std::str::FromStr;
use web3::signing::keccak256;

pub async fn handle_random(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let conn = server_data.db_connection.lock().await;

    let mut category = extract_url_param(&request, "category")?;
    if category == Some("all".to_string()) {
        category = None
    }
    let list = fancy_list(
        &*conn,
        category,
        FancyOrderBy::Score,
        ReservedStatus::NotReserved,
        None,
        PublicKeyFilter::OnlyNull,
        1000,
    )
    .await
    .unwrap();
    let random = list.choose(&mut rand::thread_rng()).unwrap();

    Ok(HttpResponse::Ok().json(random))
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FancyProviderContractApi {
    pub address: DbAddress,
    pub salt: String,
    pub factory: DbAddress,
    pub created: NaiveDateTime,
    pub score: f64,
    pub owner: Option<String>,
    pub price: i64,
    pub category: String,
    pub job: Option<String>,
    pub prov_name: String,
    pub prov_node_id: String,
    pub prov_reward_addr: String,
    pub assigned_contracts: Vec<ContractAddressDbObj>,
}

impl PartialEq<DbAddress> for String {
    fn eq(&self, other: &DbAddress) -> bool {
        self == &other.to_string()
    }
}

pub async fn handle_my_list(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let user = login_check_fn(session)?;

    let conn = server_data.db_connection.lock().await;
    let unassigned_only = extract_url_bool_param(&request, "unassigned_only")?.unwrap_or(false);
    let mut db_trans = conn.begin().await.map_err(|e| {
        log::error!("{}", e);
        actix_web::error::ErrorInternalServerError("Error starting transaction")
    })?;
    let assignments = match get_contract_address_list(&mut *db_trans, &user.uid).await {
        Ok(assignments) => assignments,
        Err(e) => {
            log::error!("{}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };
    let fancies = match fancy_list(
        &mut *db_trans,
        None,
        FancyOrderBy::Score,
        ReservedStatus::User(user.uid.clone()),
        None,
        PublicKeyFilter::OnlyNull,
        100000000,
    )
    .await
    {
        Ok(fancies) => fancies,
        Err(e) => {
            log::error!("{}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    let mut res: Vec<FancyProviderContractApi> = Vec::with_capacity(fancies.len());

    for fancy in fancies {
        let assigned_contracts: Vec<ContractAddressDbObj> = assignments
            .iter()
            .filter(|x| x.address == fancy.address)
            .cloned()
            .collect();
        if unassigned_only && !assigned_contracts.is_empty() {
            continue;
        }
        res.push(FancyProviderContractApi {
            address: fancy.address,
            salt: fancy.salt,
            factory: fancy.factory.ok_or_else(|| {
                actix_web::error::ErrorInternalServerError(
                    "DB should return only entries with factories that are not null",
                )
            })?,
            created: fancy.created,
            score: fancy.score,
            owner: fancy.owner,
            price: fancy.price,
            category: fancy.category,
            job: fancy.job,
            prov_name: fancy.prov_name,
            prov_node_id: fancy.prov_node_id,
            prov_reward_addr: fancy.prov_reward_addr,
            assigned_contracts,
        })
    }

    match db_trans.rollback().await {
        Ok(_) => {}
        Err(e) => {
            log::error!("{}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    }

    Ok(HttpResponse::Ok().json(res))
}

pub async fn handle_public_key_list(
    server_data: web::Data<Box<ServerData>>,
    session: Session,
) -> HttpResponse {
    let user = login_check_and_get!(session);

    let conn = server_data.db_connection.lock().await;

    let res = match get_public_key_list(&*conn, Some(user.uid)).await {
        Ok(res) => res,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    HttpResponse::Ok().json(res)
}

pub async fn handle_list(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let user = get_logged_user_or_null!(session);
    let conn = server_data.db_connection.lock().await;
    let limit = extract_url_int_param(&request, "limit")?;
    let public_key_base = extract_url_param(&request, "public_key_base")?;
    let mut category = extract_url_param(&request, "category")?;
    if category == Some("all".to_string()) {
        category = None
    }
    let free = extract_url_param(&request, "free")?;
    let reserved_status = match free.unwrap_or("free".to_string()).as_str() {
        "mine" => {
            if let Some(user) = user {
                ReservedStatus::User(user.uid)
            } else {
                return Ok(HttpResponse::Unauthorized().finish());
            }
        }
        "reserved" => ReservedStatus::Reserved,
        "all" => ReservedStatus::All,
        "free" => ReservedStatus::NotReserved,
        _ => ReservedStatus::NotReserved,
    };
    let order = extract_url_param(&request, "order")?.unwrap_or("score".to_string());
    let since = extract_url_date_param(&request, "since")?;
    let order = match order.as_str() {
        "score" => FancyOrderBy::Score,
        "created" => FancyOrderBy::Created,
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };

    log::info!(
        "category: {:?}, order: {:?}, since: {:?}",
        category,
        order,
        since
    );

    let public_key_base = match public_key_base {
        Some(base) => PublicKeyFilter::Selected(base),
        None => PublicKeyFilter::All,
    };

    let list = match fancy_list(
        &*conn,
        category,
        order,
        reserved_status,
        since,
        public_key_base,
        limit.unwrap_or(100),
    )
    .await
    {
        Ok(list) => list,
        Err(e) => {
            log::error!("{}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    Ok(HttpResponse::Ok().json(list))
}

pub async fn handle_fancy_estimate_total_hash(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let since = extract_url_date_param(&request, "since")?;
    let public_key_base = extract_url_param(&request, "public_key_base")?;
    let public_key_base_filter = match public_key_base {
        Some(base) => PublicKeyFilter::Selected(base),
        None => PublicKeyFilter::All,
    };
    let fancies = {
        let conn = server_data.db_connection.lock().await;
        match fancy_list(
            &*conn,
            Some("leading_zeroes".to_string()),
            FancyOrderBy::Score,
            ReservedStatus::All,
            since,
            public_key_base_filter,
            100000000,
        )
        .await
        {
            Ok(fancies) => fancies,
            Err(e) => {
                log::error!("{}", e);
                return Ok(HttpResponse::InternalServerError().finish());
            }
        }
    };

    let mut number_of_events = 0;
    #[allow(clippy::collapsible_if)]
    for fancy in fancies {
        if fancy.category == "leading_zeroes" {
            if fancy.score > 1E11 {
                number_of_events += 1;
            }
        }
    }
    Ok(HttpResponse::Ok().json(json!(
        {
            "eventDifficulty": 1.0E10f64,
            "numberOfEvents": number_of_events,
            "estimatedWorkTH": number_of_events as f64 * 1E11 / 1_000_000_000_000.0
        }
    )))
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewData {
    pub salt: String,
    pub factory: String,
    pub address: String,
    pub job_id: Option<String>,
}

pub async fn handle_fancy_new_many(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<Vec<AddNewData>>,
) -> HttpResponse {
    let mut total_score = 0.0;
    for data in new_data.iter() {
        let resp = _handle_fancy_new(
            server_data.clone(),
            web::Json(data.clone()),
            &mut total_score,
        )
        .await;
        if !resp.status().is_success() {
            return resp;
        }
    }
    HttpResponse::Ok().json(json!({
        "totalScore": total_score
    }))
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewDataEntry {
    pub salt: String,
    pub factory: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiMinerInfo {
    pub prov_node_id: Option<DbAddress>,
    pub prov_reward_addr: Option<DbAddress>,
    pub prov_name: Option<String>,
    pub prov_extra_info: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobWithMinerApi {
    pub uid: String,
    pub cruncher_ver: String,
    pub started_at: NaiveDateTime,
    pub finished_at: Option<NaiveDateTime>,
    pub requestor_id: Option<DbAddress>,
    pub hashes_reported: f64,
    pub hashes_accepted: f64,
    pub cost_reported: f64,
    pub miner: ApiMinerInfo,
    pub job_extra_info: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReportedExtraInfo {
    pub job_id: String,
    pub reported_hashes: f64,
    pub reported_cost: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewData2 {
    pub data: Vec<AddNewDataEntry>,
    pub extra: ReportedExtraInfo,
}

pub async fn handle_fancy_new_many2(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<AddNewData2>,
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
    let find_job = match fancy_get_job_info(&mut *db_trans, &new_data.extra.job_id).await {
        Ok(job) => job,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    for data in new_data.data.iter() {
        let new_data = AddNewData {
            salt: data.salt.clone(),
            factory: data.factory.clone(),
            address: data.address.clone(),
            job_id: Some(new_data.extra.job_id.clone()),
        };
        let resp =
            _handle_fancy_new_with_trans(web::Json(new_data), &mut total_score, &mut db_trans)
                .await;
        match resp {
            FancyNewResult::Ok(_ok) => {}
            FancyNewResult::Error(err) => {
                return err;
            }
            FancyNewResult::ScoreTooLow => {
                log::debug!("Score too low - skipping");
            }
        }
    }

    match fancy_update_job(
        &mut *db_trans,
        &find_job.uid,
        find_job.hashes_accepted + total_score,
        new_data.extra.reported_hashes,
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
        "totalScore": total_score
    }))
}
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewJobData {
    pub miner: ApiMinerInfo,
    pub cruncher_ver: String,
    pub requestor_id: String,
}

pub async fn handle_finish_job(
    server_data: web::Data<Box<ServerData>>,
    job_id: web::Path<String>,
) -> HttpResponse {
    let conn = server_data.db_connection.lock().await;
    let mut db_trans = match conn.begin().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let job_id = job_id.into_inner();

    match fancy_finish_job(&mut *db_trans, &job_id).await {
        Ok(_) => {
            log::info!("Job {} finished", job_id);
        }
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    let info = match fancy_get_job_info(&mut *db_trans, &job_id).await {
        Ok(info) => {
            log::info!("Job {} finished", job_id);
            info
        }
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let miner_info = match fancy_get_miner_info(&mut *db_trans, &info.miner).await {
        Ok(Some(miner_info)) => miner_info,
        Ok(None) => {
            log::error!("Miner info not found for job {}", job_id);
            return HttpResponse::InternalServerError().finish();
        }
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let api_resp = JobWithMinerApi {
        uid: info.uid,
        cruncher_ver: info.cruncher_ver,
        started_at: info.started_at,
        finished_at: info.finished_at,
        requestor_id: info.requestor_id,
        hashes_reported: info.hashes_reported,
        hashes_accepted: info.hashes_accepted,
        cost_reported: info.cost_reported,
        miner: ApiMinerInfo {
            prov_node_id: miner_info.prov_node_id,
            prov_reward_addr: miner_info.prov_reward_addr,
            prov_name: miner_info.prov_name,
            prov_extra_info: miner_info.prov_extra_info,
        },
        job_extra_info: info.job_extra_info,
    };

    match db_trans.commit().await {
        Ok(_) => HttpResponse::Ok().json(api_resp),
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn handle_new_job(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<AddNewJobData>,
) -> HttpResponse {
    let conn = server_data.db_connection.lock().await;
    let mut db_trans = match conn.begin().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    if new_data.miner.prov_node_id.is_none()
        && new_data.miner.prov_reward_addr.is_none()
        && new_data.miner.prov_name.is_none()
        && new_data.miner.prov_extra_info.is_none()
    {
        return HttpResponse::BadRequest().body("Empty miner info");
    }

    let prov_extra_info_hash = new_data
        .miner
        .prov_extra_info
        .as_ref()
        .map(|id| keccak256(id.as_bytes()));
    let prov_name_hash = new_data
        .miner
        .prov_name
        .as_ref()
        .map(|id| keccak256(id.as_bytes()));
    let prov_node_id_hash = new_data
        .miner
        .prov_node_id
        .as_ref()
        .map(|id| keccak256(id.to_string().as_bytes()));
    let prov_reward_addr_hash = new_data
        .miner
        .prov_reward_addr
        .as_ref()
        .map(|id| keccak256(id.to_string().as_bytes()));

    //xor all
    let mut xor = [0u8; 32];
    for i in 0..32 {
        if let Some(name) = prov_name_hash {
            xor[i] ^= name[i];
        }
        if let Some(extra) = prov_extra_info_hash {
            xor[i] ^= extra[i];
        }
        if let Some(node_id) = prov_node_id_hash {
            xor[i] ^= node_id[i];
        }
        if let Some(reward_addr) = prov_reward_addr_hash {
            xor[i] ^= reward_addr[i];
        }
    }
    let xor = hex::encode(xor);

    let miner_info = match fancy_get_miner_info(&mut *db_trans, &xor).await {
        Ok(Some(miner_info)) => miner_info,
        Ok(None) => {
            let new_miner_info = MinerDbObj {
                uid: xor,
                prov_node_id: new_data.miner.prov_node_id,
                prov_reward_addr: new_data.miner.prov_reward_addr,
                prov_name: new_data.miner.prov_name.clone(),
                prov_extra_info: new_data.miner.prov_extra_info.clone(),
            };
            match fancy_insert_miner_info(&mut *db_trans, new_miner_info).await {
                Ok(inserted) => inserted,
                Err(e) => {
                    log::error!("{}", e);
                    return HttpResponse::InternalServerError().finish();
                }
            }
        }
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    //generate random uuid
    let new_uid = format!("jid{:0>20}", thread_rng().next_u64());

    let requestor_id = match DbAddress::from_str(&new_data.requestor_id) {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::BadRequest()
                .body("Invalid requestor id format. Has to be ETH address");
        }
    };
    //todo implement adding job info
    let job_info = JobDbObj {
        uid: new_uid,
        cruncher_ver: new_data.cruncher_ver.clone(),
        started_at: chrono::Utc::now().naive_utc(),
        finished_at: None,
        requestor_id: Some(requestor_id),
        hashes_reported: 0.0,
        hashes_accepted: 0.0,
        cost_reported: 0.0,
        miner: miner_info.uid,
        job_extra_info: None,
    };
    let job_info = match fancy_insert_job_info(&mut *db_trans, job_info).await {
        Ok(job_info) => job_info,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let job_with_miner = JobWithMinerApi {
        uid: job_info.uid,
        cruncher_ver: job_info.cruncher_ver,
        started_at: job_info.started_at,
        finished_at: job_info.finished_at,
        requestor_id: job_info.requestor_id,
        hashes_reported: job_info.hashes_reported,
        hashes_accepted: job_info.hashes_accepted,
        cost_reported: job_info.cost_reported,
        miner: new_data.miner.clone(),
        job_extra_info: job_info.job_extra_info,
    };
    match db_trans.commit().await {
        Ok(_) => HttpResponse::Ok().json(job_with_miner),
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

async fn _handle_fancy_new(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<AddNewData>,
    total_score: &mut f64,
) -> HttpResponse {
    let result = if new_data.factory.len() == 42 || new_data.factory.len() == 40 {
        let factory = match web3::types::Address::from_str(&new_data.factory) {
            Ok(factory) => factory,
            Err(e) => {
                log::error!("{}", e);
                return HttpResponse::BadRequest().finish();
            }
        };
        match parse_fancy(new_data.salt.clone(), factory) {
            Ok(fancy) => fancy,
            Err(e) => {
                log::error!("{}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        match parse_fancy_private(new_data.factory.clone(), new_data.salt.clone()) {
            Ok(fancy) => fancy,
            Err(e) => {
                log::error!("{}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    };

    if result.score < 1E10 {
        log::error!("Score too low: {}", result.score);
        return HttpResponse::Ok().body("Score too low");
    }

    if format!("{:#x}", result.address.addr()) != new_data.address.to_lowercase() {
        log::error!(
            "Address mismatch expected: {}, got: {}",
            format!("{:#x}", result.address.addr()),
            new_data.address.to_lowercase()
        );
        return HttpResponse::BadRequest().body("Address mismatch");
    }
    let score = result.score;
    let conn = server_data.db_connection.lock().await;
    let mut db_trans = match conn.begin().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    };

    //result.job = None;

    match insert_fancy_obj(&mut *db_trans, result).await {
        Ok(_) => match db_trans.commit().await {
            Ok(_) => {
                *total_score += score;
                HttpResponse::Ok().json(json!({
                    "totalSore": score
                }))
            }
            Err(e) => {
                log::error!("{}", e);
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                HttpResponse::Ok().body("Already exists")
            } else {
                log::error!("{}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

enum FancyNewResult {
    Ok(HttpResponse),
    Error(HttpResponse),
    ScoreTooLow,
}

async fn _handle_fancy_new_with_trans(
    new_data: web::Json<AddNewData>,
    total_score: &mut f64,
    db_trans: &mut Transaction<'_, Sqlite>,
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
        let public_key_bytes = match hex::decode(public_key_base.clone()) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error!("{}", e);
                return FancyNewResult::Error(HttpResponse::BadRequest().finish());
            }
        };
        if public_key_bytes.len() != 64 {
            log::error!("Invalid public key length: {}", public_key_base);
            return FancyNewResult::Error(HttpResponse::BadRequest().finish());
        }
        let public_key_base = "0x".to_string() + &hex::encode(public_key_bytes);
        let fancy = match parse_fancy_private(public_key_base, new_data.salt.clone()) {
            Ok(fancy) => fancy,
            Err(e) => {
                log::error!("{}", e);
                return FancyNewResult::Error(HttpResponse::InternalServerError().finish());
            }
        };
        let public_key_base = match fancy.public_key_base.clone() {
            Some(key) => key,
            None => {
                log::error!("Public key not found after parse");
                return FancyNewResult::Error(HttpResponse::InternalServerError().finish());
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

    result.job = new_data.job_id.clone();

    if result.score < 1E10 {
        log::error!("Score too low: {}", result.score);
        return FancyNewResult::ScoreTooLow;
    }

    if format!("{:#x}", result.address.addr()) != new_data.address.to_lowercase() {
        log::error!(
            "Address mismatch expected: {}, got: {}",
            format!("{:#x}", result.address.addr()),
            new_data.address.to_lowercase()
        );
        return FancyNewResult::Error(HttpResponse::BadRequest().body("Address mismatch"));
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
                FancyNewResult::Ok(HttpResponse::Ok().body("Already exists"))
            } else {
                log::error!("{}", e);
                FancyNewResult::Error(HttpResponse::InternalServerError().finish())
            }
        }
    }
}

pub async fn handle_fancy_new(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<AddNewData>,
) -> HttpResponse {
    let mut total_score = 0.0;
    _handle_fancy_new(server_data, new_data, &mut total_score).await
}

pub async fn handle_fancy_deploy_start(
    server_data: web::Data<Box<ServerData>>,
    contract_id: web::Path<String>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);
    let contract_id = contract_id.into_inner();

    let conn = server_data.db_connection.lock().await;

    let contract = match get_contract_by_id(&*conn, contract_id, user.uid.clone()).await {
        Ok(Some(contract)) => {
            let mut contract = contract;
            match contract.deploy_status {
                DeployStatus::None => {
                    contract.deploy_status = DeployStatus::Requested;
                    contract
                }
                DeployStatus::Requested => return HttpResponse::Ok().body("Already requested"),
                DeployStatus::TxSent => return HttpResponse::Ok().body("Already sent"),
                DeployStatus::Failed => return HttpResponse::Ok().body("Deployment Failed"),
                DeployStatus::Succeeded => return HttpResponse::Ok().body("Deployment Succeeded"),
            }
        }
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    match update_contract_data(&*conn, contract).await {
        Ok(contr) => HttpResponse::Ok().json(contr),
        Err(err) => {
            log::error!("Error updating contract data {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}
pub async fn handle_fancy_buy_api(
    server_data: web::Data<Box<ServerData>>,
    address: web::Path<String>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);

    let address = address.into_inner();

    let conn = server_data.db_connection.lock().await;

    let mut trans = match conn.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            log::error!("Error starting transaction: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let user_for_tx = match get_user(&mut *trans, &user.email).await {
        Ok(user) => user,
        Err(err) => {
            log::error!("Error getting user: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let address = normalize_address!(address);
    let address_db = match fancy_get_by_address(&mut *trans, address).await {
        Ok(Some(addr)) => addr,
        Ok(None) => {
            log::error!("Address not found: {}", address);
            return HttpResponse::NotFound().finish();
        }
        Err(err) => {
            log::error!("Error getting address: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    if address_db.owner.is_some() {
        log::error!("Address already owned: {}", address);
        return HttpResponse::BadRequest().body("Address already owned");
    }

    if user_for_tx.tokens < address_db.price {
        log::error!(
            "User has insufficient funds: {} < {}",
            user_for_tx.tokens,
            address_db.price
        );
        return HttpResponse::BadRequest().body("Insufficient funds");
    }

    match fancy_update_owner(&mut *trans, address, user.uid.clone()).await {
        Ok(_) => {}
        Err(err) => {
            log::error!("Error updating owner: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    }

    let tokens_left = user_for_tx.tokens - address_db.price;
    log::info!(
        "User {} bought address {} for {}, tokens left: {}",
        user.email,
        address,
        address_db.price,
        tokens_left
    );
    match update_user_tokens(&mut *trans, &user.email, tokens_left).await {
        Ok(_) => {}
        Err(err) => {
            log::error!("Error updating user tokens: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    }

    match trans.commit().await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            log::error!("Error committing transaction: {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}
