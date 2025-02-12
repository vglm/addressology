pub mod score;
pub mod tokens;

use crate::api::utils::{extract_url_date_param, extract_url_int_param, extract_url_param};
use crate::db::model::{DeployStatus, MinerDbObj, UserDbObj};
use crate::db::ops::{
    fancy_get_by_address, fancy_get_miner_info, fancy_insert_miner_info, fancy_list_all_free,
    fancy_list_best_score, fancy_list_newest, fancy_update_owner, get_contract_by_id, get_user,
    insert_fancy_obj, update_contract_data, update_user_tokens, FancyOrderBy,
};
use crate::fancy::parse_fancy;
use crate::{login_check_and_get, normalize_address, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    let list = fancy_list_best_score(&conn, category, FancyOrderBy::Score, None, 1000)
        .await
        .unwrap();
    let random = list.choose(&mut rand::thread_rng()).unwrap();

    Ok(HttpResponse::Ok().json(random))
}

pub async fn handle_list(server_data: web::Data<Box<ServerData>>) -> impl Responder {
    let conn = server_data.db_connection.lock().await;
    let list = fancy_list_all_free(&conn).await.unwrap();

    HttpResponse::Ok().json(list)
}

pub async fn handle_list_newest(server_data: web::Data<Box<ServerData>>) -> impl Responder {
    let conn = server_data.db_connection.lock().await;
    let list = fancy_list_newest(&conn).await.unwrap();

    HttpResponse::Ok().json(list)
}

pub async fn handle_list_best_score(
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
        match fancy_list_best_score(&conn, category, order, since, limit.unwrap_or(100)).await {
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
        match fancy_list_best_score(
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
    pub miner: String,
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
    pub cruncher_ver: String,
    pub mined_by: String,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewDataMinerInfo {
    pub cruncher_ver: String,
    pub mined_by: String,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
    pub job_id: Option<String>,
    pub requestor_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewData2 {
    pub data: Vec<AddNewDataEntry>,
    pub miner: AddNewDataMinerInfo,
}

pub async fn handle_fancy_new_many2(
    server_data: web::Data<Box<ServerData>>,
    new_data: web::Json<AddNewData2>,
) -> HttpResponse {
    let mut total_score = 0.0;
    for data in new_data.data.iter() {
        let new_data = AddNewData {
            salt: data.salt.clone(),
            miner: new_data.miner.cruncher_ver.clone(),
            factory: data.factory.clone(),
            address: data.address.clone(),
            job_id: None,
        };
        let resp =
            _handle_fancy_new(server_data.clone(), web::Json(new_data), &mut total_score).await;
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
pub struct AddNewJobData {
    pub miner: ApiMinerInfo,
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

    let cruncher_ver_hash = keccak256(new_data.miner.cruncher_ver.as_bytes());
    let mined_by_hash = keccak256(new_data.miner.mined_by.as_bytes());
    let provider_id_hash = new_data
        .miner
        .provider_id
        .as_ref()
        .map(|id| keccak256(id.as_bytes()));
    let provider_name_hash = new_data
        .miner
        .provider_name
        .as_ref()
        .map(|name| keccak256(name.as_bytes()));

    //xor all
    let mut xor = [0u8; 32];
    for i in 0..32 {
        xor[i] = cruncher_ver_hash[i] ^ mined_by_hash[i];
        if let Some(id) = provider_id_hash {
            xor[i] ^= id[i];
        }
        if let Some(name) = provider_name_hash {
            xor[i] ^= name[i];
        }
    }
    let xor = hex::encode(xor);

    let miner_info = match fancy_get_miner_info(&mut *db_trans, &xor).await {
        Ok(Some(miner_info)) => miner_info,
        Ok(None) => {
            let new_miner_info = MinerDbObj {
                uid: xor,
                cruncher_ver: new_data.miner.cruncher_ver.clone(),
                mined_by: new_data.miner.mined_by.clone(),
                provider_id: new_data.miner.provider_id.clone(),
                provider_name: new_data.miner.provider_name.clone(),
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
    //todo implement adding job info
    //let job_info =

    HttpResponse::Ok().finish()
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
