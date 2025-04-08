use crate::db::model::{DeployStatus, UserDbObj};
use crate::db::ops::{get_contract_by_id, update_contract_data};
use crate::{login_check_and_get, ServerData};
use actix_session::Session;
use actix_web::{web, HttpResponse};

pub async fn handle_fancy_deploy_start(
    server_data: web::Data<Box<ServerData>>,
    contract_id: web::Path<String>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);
    let contract_id = contract_id.into_inner();

    let conn = server_data.db_connection.lock().await;

    let contract = match get_contract_by_id(&*conn, contract_id, user.uid).await {
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
