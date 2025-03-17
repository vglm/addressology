use crate::api::fancy::ApiMinerInfo;
use crate::config::get_base_difficulty_price;
use crate::db::model::FancyScore;
use crate::db::model::UserDbObj;
use crate::db::ops::{fancy_get_by_address, fancy_get_job_info, fancy_get_miner_info};
use crate::fancy::{list_score_categories, score_fancy};
use crate::{get_logged_user_or_null, normalize_address, ServerData};
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
    factory: Option<String>,
    salt: Option<String>,
    public_key_base: Option<String>,
}

//this request can be public
pub async fn handle_score_custom(
    server_data: web::Data<Box<ServerData>>,
    session: actix_session::Session,
    address: web::Path<String>,
) -> HttpResponse {
    let user = get_logged_user_or_null!(session);
    let address = normalize_address!(address.into_inner());

    if let Some(_user) = user {
        //@todo filter out sensitive user data
        let db = server_data.db_connection.lock().await;

        let fancy = fancy_get_by_address(&*db, address)
            .await
            .unwrap_or_default();

        let score = score_fancy(address.addr());

        let api_miner_info = if let Some(fancy) = &fancy {
            if let Some(job_id) = fancy.job_id {
                let job = match fancy_get_job_info(&*db, job_id).await {
                    Ok(job) => job,
                    Err(e) => {
                        log::error!("Error getting job info: {}", e);
                        return HttpResponse::InternalServerError().finish();
                    }
                };
                match fancy_get_miner_info(&*db, &job.miner).await {
                    Ok(miner) => miner,
                    Err(e) => {
                        log::error!("Error getting miner info: {}", e);
                        return HttpResponse::InternalServerError().finish();
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        HttpResponse::Ok().json(FancyScoreResponse {
            score: score.clone(),
            price: (get_base_difficulty_price() as f64 * score.price_multiplier) as i64,
            miner_info: api_miner_info.map(|m| ApiMinerInfo {
                prov_node_id: m.prov_node_id,
                prov_reward_addr: m.prov_reward_addr,
                prov_name: m.prov_name,
                prov_extra_info: m.prov_extra_info,
            }),
            mined: fancy.as_ref().map(|f| f.created),
            factory: fancy
                .as_ref()
                .and_then(|f| f.factory.map(|f| f.to_string())),
            salt: fancy.as_ref().map(|f| f.salt.clone()),
            public_key_base: fancy.as_ref().and_then(|f| f.public_key_base.clone()),
        })
    } else {
        let score = score_fancy(address.addr());

        HttpResponse::Ok().json(FancyScoreResponse {
            score: score.clone(),
            price: (get_base_difficulty_price() as f64 * score.price_multiplier) as i64,
            miner_info: None,
            mined: None,
            factory: None,
            salt: None,
            public_key_base: None,
        })
    }
}

pub async fn handle_get_score_categories() -> HttpResponse {
    HttpResponse::Ok().json(list_score_categories())
}
