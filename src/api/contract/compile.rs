use crate::solc::compile_solc;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ServerData;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompileData {
    pub sources: BTreeMap<String, String>,
}

pub async fn handle_compile(
    server_data: web::Data<Box<ServerData>>,
    deploy_data: web::Json<CompileData>,
) -> HttpResponse {
    let _conn = server_data.db_connection.lock().await;

    log::info!("Compiling contract: {:#?}", deploy_data.sources);
    match compile_solc(deploy_data.sources.clone(), "0.8.28").await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => {
            log::error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
