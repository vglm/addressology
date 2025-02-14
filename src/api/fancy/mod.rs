pub mod score;
pub mod tokens;

use crate::api::utils::{extract_url_date_param, extract_url_int_param, extract_url_param};
use crate::db::model::{DeployStatus, JobDbObj, MinerDbObj, UserDbObj};
use crate::db::ops::{fancy_finish_job, fancy_get_by_address, fancy_get_job_info, fancy_get_miner_info, fancy_insert_job_info, fancy_insert_miner_info, fancy_list, fancy_update_job, fancy_update_owner, get_contract_by_id, get_user, insert_fancy_obj, update_contract_data, update_user_tokens, FancyOrderBy};
use crate::fancy::parse_fancy;
use crate::types::DbAddress;
use crate::{login_check_and_get, normalize_address, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use pbkdf2::password_hash::rand_core::RngCore;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Executor, Sqlite};
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
    let list = fancy_list(&conn, category, FancyOrderBy::Score, None, 1000)
        .await
        .unwrap();
    let random = list.choose(&mut rand::thread_rng()).unwrap();

    Ok(HttpResponse::Ok().json(random))
}


pub async fn handle_list(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let conn = server_data.db_connection.lock().await;
    let limit = extract_url_int_param(&request, "limit")?;
    let mut category = extract_url_param(&request, "category")?;
    if category == Some("all".to_string()) {
        category = None
    }
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

    let list =
        match fancy_list(&conn, category, order, since, limit.unwrap_or(100)).await {
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
    let fancies = {
        let conn = server_data.db_connection.lock().await;
        match fancy_list(
            &conn,
            Some("leading_zeroes".to_string()),
            FancyOrderBy::Score,
            since,
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
            _handle_fancy_new_with_trans(web::Json(new_data), &mut total_score, &mut *db_trans)
                .await;
        if !resp.status().is_success() {
            return resp;
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
    let factory = match web3::types::Address::from_str(&new_data.factory) {
        Ok(factory) => factory,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::BadRequest().finish();
        }
    };
    let result = match parse_fancy(new_data.salt.clone(), factory) {
        Ok(fancy) => fancy,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
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

async fn _handle_fancy_new_with_trans<'c, E>(
    new_data: web::Json<AddNewData>,
    total_score: &mut f64,
    db_trans: E,
) -> HttpResponse
where
    E: Executor<'c, Database = Sqlite>,
{
    let factory = match web3::types::Address::from_str(&new_data.factory) {
        Ok(factory) => factory,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::BadRequest().finish();
        }
    };
    let mut result = match parse_fancy(new_data.salt.clone(), factory) {
        Ok(fancy) => fancy,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    result.job = new_data.job_id.clone();

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

    match insert_fancy_obj(db_trans, result).await {
        Ok(_) => {
            *total_score += score;
            HttpResponse::Ok().json(json!({
                "totalSore": score
            }))
        }
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
