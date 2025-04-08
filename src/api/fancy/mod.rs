pub mod buy;
pub mod deploy;
pub mod estimate;
pub mod job;
pub mod list;
pub mod my;
pub mod new;
pub mod score;
pub mod tokens;

use crate::api::utils::extract_url_param;
use crate::db::model::UserDbObj;
use crate::db::ops::{
    fancy_list, get_public_key_list, FancyOrderBy, PublicKeyFilter, ReservedStatus,
};
use crate::types::DbAddress;
use crate::{login_check_and_get, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;

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
    let random = list.choose(&mut rand::rng()).unwrap();

    Ok(HttpResponse::Ok().json(random))
}

impl PartialEq<DbAddress> for String {
    fn eq(&self, other: &DbAddress) -> bool {
        self == &other.to_string()
    }
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiMinerInfo {
    pub prov_node_id: Option<DbAddress>,
    pub prov_reward_addr: Option<DbAddress>,
    pub prov_name: Option<String>,
    pub prov_extra_info: Option<String>,
}
