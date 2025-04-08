use crate::api::fancy::ApiMinerInfo;
use crate::api::utils::{extract_url_date_param, extract_url_int_param, extract_url_param};
use crate::db::model::{JobDbObj, MinerDbObj, UserDbObj};
use crate::db::ops::{
    fancy_finish_job, fancy_get_job_info, fancy_get_miner_info, fancy_insert_job_info,
    fancy_insert_miner_info, fancy_job_list, FancyJobOrderBy, FancyJobStatus,
};
use crate::types::DbAddress;
use crate::{get_logged_user_or_null, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;

use rand::{rng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::cmp::PartialEq;
use web3::signing::keccak256;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddNewJobData {
    pub miner: ApiMinerInfo,
    pub cruncher_ver: String,
    pub requestor_id: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobWithMinerApi {
    pub uid: Uuid,
    pub cruncher_ver: String,
    pub started_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub finished_at: Option<NaiveDateTime>,
    pub requestor_id: Option<DbAddress>,
    pub hashes_reported: f64,
    pub hashes_accepted: f64,
    pub entries_accepted: i64,
    pub entries_rejected: i64,
    pub cost_reported: f64,
    pub miner: ApiMinerInfo,
    pub job_extra_info: Option<String>,
}

pub async fn handle_job_list(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let _user = get_logged_user_or_null!(session);
    let conn = server_data.db_connection.lock().await;
    let limit = extract_url_int_param(&request, "limit")?;

    let order = extract_url_param(&request, "order")?.unwrap_or("score".to_string());
    let status = extract_url_param(&request, "status")?;
    let since = extract_url_date_param(&request, "since")?;

    let requestor_id = extract_url_param(&request, "requestor_id")?;
    let order = match order.as_str() {
        "created" => FancyJobOrderBy::Date,
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };
    let status = match status.unwrap_or("all".to_string()).as_str() {
        "all" => FancyJobStatus::All,
        "finished" => FancyJobStatus::Finished,
        "active" => FancyJobStatus::Active,
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };
    let requestor_id = match requestor_id {
        Some(id) => Some(DbAddress::from_str(&id).map_err(|_| {
            actix_web::error::ErrorBadRequest("Invalid requestor id format. Has to be ETH address")
        })?),
        None => None,
    };

    let list = match fancy_job_list(
        &*conn,
        order,
        since,
        status,
        requestor_id,
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

    let job_id = match Uuid::parse_str(&job_id) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::BadRequest().finish();
        }
    };

    match fancy_finish_job(&mut *db_trans, job_id).await {
        Ok(_) => {
            log::info!("Job {} finished", job_id);
        }
        Err(e) => {
            log::error!("{}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    let info = match fancy_get_job_info(&mut *db_trans, job_id).await {
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
        updated_at: info.updated_at,
        finished_at: info.finished_at,
        requestor_id: info.requestor_id,
        hashes_reported: info.hashes_reported,
        hashes_accepted: info.hashes_accepted,
        entries_accepted: info.entries_accepted,
        entries_rejected: info.entries_rejected,
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
    let new_uid = Uuid::new_v4();

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
        updated_at: chrono::Utc::now().naive_utc(),
        finished_at: None,
        requestor_id: Some(requestor_id),
        hashes_reported: 0.0,
        hashes_accepted: 0.0,
        entries_accepted: 0,
        entries_rejected: 0,
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
        updated_at: job_info.updated_at,
        finished_at: job_info.finished_at,
        requestor_id: job_info.requestor_id,
        hashes_reported: job_info.hashes_reported,
        hashes_accepted: job_info.hashes_accepted,
        entries_accepted: job_info.entries_accepted,
        entries_rejected: job_info.entries_rejected,
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
