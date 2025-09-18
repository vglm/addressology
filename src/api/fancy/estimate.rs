use crate::api::utils::{extract_url_date_param, extract_url_param};
use crate::db::ops::{fancy_list, FancyOrderBy, PublicKeyFilter, ReservedStatus};
use crate::ServerData;
use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;

pub async fn handle_fancy_estimate_total_hash(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let since = extract_url_date_param(&request, "since")?;
    let public_key_base = extract_url_param(&request, "public_key_base")?;
    let provider_id = extract_url_param(&request, "provider_id")?;
    let public_key_base_filter = match public_key_base {
        Some(base) => PublicKeyFilter::Selected(base),
        None => PublicKeyFilter::All,
    };
    let fancies = {
        let conn = server_data.db_connection.lock().await;
        match fancy_list(
            &*conn,
            Some("leading_zeroes".to_string()),
            FancyOrderBy::Score,
            ReservedStatus::All,
            since,
            provider_id,
            public_key_base_filter,
            100000000,
        )
        .await
        {
            Ok(fancies) => fancies,
            Err(e) => {
                log::error!("{}", e);
                return Ok(HttpResponse::InternalServerError().finish());
            }
        }
    };

    let mut number_of_events = 0;
    #[allow(clippy::collapsible_if)]
    for fancy in fancies {
        if fancy.category == "leading_zeroes" {
            if fancy.score > 1E11 {
                number_of_events += 1;
            }
        }
    }
    Ok(HttpResponse::Ok().json(json!(
        {
            "eventDifficulty": 1.0E10f64,
            "numberOfEvents": number_of_events,
            "estimatedWorkTH": number_of_events as f64 * 1E11 / 1_000_000_000_000.0
        }
    )))
}
