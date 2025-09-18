use crate::api::utils::{extract_url_date_param, extract_url_int_param, extract_url_param};
use crate::db::model::UserDbObj;
use crate::db::ops::{fancy_list, FancyOrderBy, PublicKeyFilter, ReservedStatus};
use crate::{get_logged_user_or_null, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};

pub async fn handle_list(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let user = get_logged_user_or_null!(session);
    let conn = server_data.db_connection.lock().await;
    let limit = extract_url_int_param(&request, "limit")?;
    let public_key_base = extract_url_param(&request, "public_key_base")?;
    let mut category = extract_url_param(&request, "category")?;
    if category == Some("all".to_string()) {
        category = None
    }
    let provider_id = extract_url_param(&request, "provider_id")?;
    let free = extract_url_param(&request, "free")?;
    let reserved_status = match free.unwrap_or("free".to_string()).as_str() {
        "mine" => {
            if let Some(user) = user {
                ReservedStatus::User(user.uid)
            } else {
                return Ok(HttpResponse::Unauthorized().finish());
            }
        }
        "reserved" => ReservedStatus::Reserved,
        "all" => ReservedStatus::All,
        "free" => ReservedStatus::NotReserved,
        _ => ReservedStatus::NotReserved,
    };
    let order = extract_url_param(&request, "order")?.unwrap_or("score".to_string());
    let since = extract_url_date_param(&request, "since")?;
    let order = match order.as_str() {
        "score" => FancyOrderBy::Score,
        "created" => FancyOrderBy::Created,
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };

    let public_key_base = match public_key_base {
        Some(base) => PublicKeyFilter::Selected(base),
        None => PublicKeyFilter::OnlyNull,
    };

    let list = match fancy_list(
        &*conn,
        category,
        order,
        reserved_status,
        since,
        provider_id,
        public_key_base,
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
