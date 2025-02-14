use actix_web::HttpRequest;
use chrono::{DateTime, Utc};
use percent_encoding::percent_decode_str;

pub fn extract_url_param(
    request: &HttpRequest,
    param: &str,
) -> Result<Option<String>, actix_web::Error> {
    for (key, value) in request.query_string().split('&').map(|x| {
        let mut split = x.split('=');
        (split.next().unwrap_or(""), split.next().unwrap_or(""))
    }) {
        if key == param {
            return match percent_decode_str(value).decode_utf8() {
                Ok(val) => Ok(Some(val.to_string())),
                Err(err) => Err(actix_web::error::ErrorBadRequest(err)),
            };
        }
    }
    Ok(None)
}

//probably nice to make generic version of this, but for now i64 is enough
pub fn extract_url_int_param(
    request: &HttpRequest,
    param: &str,
) -> Result<Option<i64>, actix_web::Error> {
    if let Some(str) = extract_url_param(request, param)? {
        match str.parse::<i64>() {
            Ok(val) => Ok(Some(val)),
            Err(_) => Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to parse {} as i64",
                param
            ))),
        }
    } else {
        Ok(None)
    }
}

pub fn extract_url_date_param(
    request: &HttpRequest,
    param: &str,
) -> Result<Option<chrono::DateTime<Utc>>, actix_web::Error> {
    if let Some(str) = extract_url_param(request, param)? {
        match str.parse::<DateTime<Utc>>() {
            Ok(val) => Ok(Some(val)),
            Err(_) => Err(actix_web::error::ErrorBadRequest(format!(
                "Failed to parse {} as date",
                param
            ))),
        }
    } else {
        Ok(None)
    }
}
