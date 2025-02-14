use crate::api::fancy::ApiMinerInfo;
use crate::config::get_base_difficulty_price;
use crate::db::model::FancyScore;
use crate::db::ops::fancy_get_by_address;
use crate::fancy::{list_score_categories, score_fancy};
use crate::{normalize_address, ServerData};
use actix_web::{web, HttpResponse};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct FancyScoreResponse {
    score: FancyScore,
    price: i64,
    miner_info: Option<ApiMinerInfo>,
    mined: Option<NaiveDateTime>,
}

//this request can be public
pub async fn handle_score_custom(
    server_data: web::Data<Box<ServerData>>,
    address: web::Path<String>,
) -> HttpResponse {
    let address = normalize_address!(address.into_inner());

    let db = server_data.db_connection.lock().await;
    let fancy = fancy_get_by_address(&*db, address)
        .await
        .unwrap_or_default();

    let score = score_fancy(address.addr());

    HttpResponse::Ok().json(FancyScoreResponse {
        score: score.clone(),
        price: (get_base_difficulty_price() as f64 * score.price_multiplier) as i64,
        miner_info: None,
        mined: fancy.map(|f| f.created),
    })
}

pub async fn handle_get_score_categories() -> HttpResponse {
    HttpResponse::Ok().json(list_score_categories())
}
