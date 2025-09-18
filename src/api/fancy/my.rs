use crate::api::contract::api::login_check_fn;
use crate::api::utils::extract_url_bool_param;
use crate::db::model::ContractAddressDbObj;
use crate::db::ops::{
    fancy_list, get_contract_address_list, FancyOrderBy, PublicKeyFilter, ReservedStatus,
};
use crate::types::DbAddress;
use crate::ServerData;
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FancyProviderContractApi {
    pub address: DbAddress,
    pub salt: String,
    pub factory: DbAddress,
    pub created: NaiveDateTime,
    pub score: f64,
    pub owner_id: Option<Uuid>,
    pub price: i64,
    pub category: String,
    pub job_id: Option<Uuid>,
    pub prov_name: String,
    pub prov_node_id: Option<DbAddress>,
    pub prov_reward_addr: Option<DbAddress>,
    pub assigned_contracts: Vec<ContractAddressDbObj>,
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
    let assignments = match get_contract_address_list(&mut *db_trans, user.uid).await {
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
        ReservedStatus::User(user.uid),
        None,
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
            owner_id: fancy.owner_id,
            price: fancy.price,
            category: fancy.category,
            job_id: fancy.job_id,
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
