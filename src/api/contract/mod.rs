mod api;

use crate::db::model::{ContractCreateFromApi, ContractDbObj, DeployStatus, UserDbObj};
use crate::db::ops::{
    delete_contract_by_id, get_all_contracts_by_user, get_contract_by_id, insert_contract_obj,
    update_contract_data,
};
use crate::{login_check_and_get, ServerData};
use actix_session::Session;
use actix_web::web::Data;
use actix_web::{web, HttpResponse};

pub async fn get_contract_info_api(
    data: Data<Box<ServerData>>,
    contract_id: web::Path<String>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);

    let contract_id = contract_id.into_inner();

    let db = data.db_connection.lock().await;

    match get_contract_by_id(&*db, contract_id, user.uid).await {
        Ok(contract) => HttpResponse::Ok().json(contract),
        Err(e) => {
            log::error!("Error getting scan info: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn insert_contract_info_api(
    data: Data<Box<ServerData>>,
    contract: web::Json<ContractCreateFromApi>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);

    let db = data.db_connection.lock().await;

    let contract_api = contract.into_inner();
    let contract = ContractDbObj {
        contract_id: uuid::Uuid::new_v4().to_string(),
        user_id: user.uid,
        created: chrono::Utc::now().naive_utc(),
        address: contract_api.address,
        network: contract_api.network,
        data: contract_api.data,
        tx: None,
        deploy_status: DeployStatus::None,
        deploy_requested: None,
        deploy_sent: None,
        deployed: None,
    };

    match insert_contract_obj(&db, contract).await {
        Ok(contr) => HttpResponse::Ok().json(contr),
        Err(e) => {
            log::error!("Error inserting scan info: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn update_contract_info_api(
    data: Data<Box<ServerData>>,
    contract: web::Json<ContractDbObj>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);

    let db = data.db_connection.lock().await;

    let contract = contract.into_inner();

    let mut trans = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Error starting transaction: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut new_contract_version =
        match get_contract_by_id(&mut *trans, contract.contract_id, user.uid).await {
            Ok(Some(contract)) => contract,
            Ok(None) => {
                log::error!("Contract not found");
                return HttpResponse::NotFound().finish();
            }
            Err(e) => {
                log::error!("Error getting scan info: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        };

    if new_contract_version.deploy_status != DeployStatus::None {
        log::error!("Contract is not in None state");
        return HttpResponse::BadRequest()
            .body("Contract is already sent for deployment and cannot be updated");
    }

    new_contract_version.data = contract.data;
    new_contract_version.address = contract.address;
    new_contract_version.network = contract.network;

    let contr = match update_contract_data(&mut *trans, new_contract_version).await {
        Ok(contr) => contr,
        Err(e) => {
            log::error!("Error inserting scan info: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    match trans.commit().await {
        Ok(_) => HttpResponse::Ok().json(contr),
        Err(err) => {
            log::error!("Error committing transaction: {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn get_contracts_api(data: Data<Box<ServerData>>, session: Session) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);

    let db = data.db_connection.lock().await;

    match get_all_contracts_by_user(&db, user.uid).await {
        Ok(contracts) => HttpResponse::Ok().json(contracts),
        Err(e) => {
            log::error!("Error getting scan info: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn delete_contract_api(
    data: Data<Box<ServerData>>,
    contract_id: web::Path<String>,
    session: Session,
) -> HttpResponse {
    let user: UserDbObj = login_check_and_get!(session);

    let contract_id = contract_id.into_inner();

    let db = data.db_connection.lock().await;

    let mut trans = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Error starting transaction: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let contract = match get_contract_by_id(&mut *trans, contract_id, user.uid).await {
        Ok(Some(contract)) => contract,
        Ok(None) => {
            log::error!("Contract not found");
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error getting scan info: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };
    if contract.deploy_status != DeployStatus::None {
        log::error!("Contract is not in None state");
        return HttpResponse::BadRequest()
            .body("Contract is already sent for deployment and cannot be deleted");
    }
    match delete_contract_by_id(&mut *trans, contract.contract_id, contract.user_id).await {
        Ok(_) => match trans.commit().await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => {
                log::error!("Error committing transaction: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(e) => {
            log::error!("Error deleting scan info: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
