use crate::db::model::UserDbObj;
use crate::db::ops::{fancy_get_by_address, fancy_update_owner, get_user, update_user_tokens};
use crate::{login_check_and_get, normalize_address, ServerData};
use actix_session::Session;
use actix_web::{web, HttpResponse};

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

    if address_db.owner_id.is_some() {
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

    match fancy_update_owner(&mut *trans, address, user.uid).await {
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
