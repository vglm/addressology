use crate::api::utils::{extract_url_date_param, extract_url_int_param, extract_url_param};
use crate::db::model::{FancyCategoryStatDbObj, UserDbObj};
use crate::db::ops::{
    fancy_estimated_work_events, fancy_search, fancy_stats, FancyOrderBy, FancySearchParams,
    PatternMode, PublicKeyFilter, ReservedStatus,
};
use crate::fancy::FancyScoreCategory;
use crate::{get_logged_user_or_null, ServerData};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use serde::Serialize;
use sqlx::PgPool;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const MAX_PAGE_SIZE: i64 = 200;
const MAX_OFFSET: i64 = 100_000;
const MIN_CONTAINS_PATTERN: usize = 3;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct FancySearchResponse {
    entries: Vec<crate::db::model::FancyProviderDbObj>,
    total: i64,
    total_capped: bool,
    limit: i64,
    offset: i64,
}

fn bad_request(msg: &str) -> actix_web::Error {
    actix_web::error::ErrorBadRequest(msg.to_string())
}

fn parse_pattern(
    pattern: Option<String>,
    mode: PatternMode,
) -> Result<Option<String>, actix_web::Error> {
    let Some(pattern) = pattern else {
        return Ok(None);
    };
    let pattern = pattern.trim().to_lowercase();
    let pattern = pattern.strip_prefix("0x").unwrap_or(&pattern).to_string();
    if pattern.is_empty() {
        return Ok(None);
    }
    if pattern.len() > 40 {
        return Err(bad_request(
            "Pattern too long, addresses have 40 hex digits",
        ));
    }
    if !pattern.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad_request(
            "Pattern may contain only hex digits (0-9, a-f)",
        ));
    }
    if mode != PatternMode::Prefix && pattern.len() < MIN_CONTAINS_PATTERN {
        return Err(bad_request(
            "Patterns for contains/suffix search need at least 3 hex digits",
        ));
    }
    Ok(Some(pattern))
}

async fn get_pool(server_data: &web::Data<Box<ServerData>>) -> PgPool {
    // Clone the pool out of the shared mutex so the lock is not held while queries run.
    server_data.db_connection.lock().await.clone()
}

pub async fn handle_search(
    server_data: web::Data<Box<ServerData>>,
    request: HttpRequest,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let user: Option<UserDbObj> = get_logged_user_or_null!(session);

    let mode = match extract_url_param(&request, "mode")?.as_deref() {
        None | Some("contains") => PatternMode::Contains,
        Some("prefix") => PatternMode::Prefix,
        Some("suffix") => PatternMode::Suffix,
        Some(_) => return Err(bad_request("mode must be one of prefix, contains, suffix")),
    };
    let pattern = parse_pattern(extract_url_param(&request, "q")?, mode)?;

    let category = match extract_url_param(&request, "category")? {
        None => None,
        Some(cat) if cat == "all" => None,
        Some(cat) => {
            if FancyScoreCategory::from_str(&cat).is_err() {
                return Err(bad_request("Unknown category"));
            }
            Some(cat)
        }
    };

    let reserved = match extract_url_param(&request, "free")?.as_deref() {
        None | Some("free") => ReservedStatus::NotReserved,
        Some("reserved") => ReservedStatus::Reserved,
        Some("all") => ReservedStatus::All,
        Some("mine") => {
            if let Some(user) = &user {
                ReservedStatus::User(user.uid)
            } else {
                return Ok(HttpResponse::Unauthorized().finish());
            }
        }
        Some(_) => return Err(bad_request("free must be one of free, reserved, all, mine")),
    };

    let public_key_base = match extract_url_param(&request, "key")?.as_deref() {
        None | Some("all") => PublicKeyFilter::All,
        Some("none") => PublicKeyFilter::OnlyNull,
        Some(key) => {
            let key = key.trim().to_lowercase();
            if !key
                .strip_prefix("0x")
                .unwrap_or(&key)
                .chars()
                .all(|c| c.is_ascii_hexdigit())
            {
                return Err(bad_request(
                    "key must be a hex public key base, all or none",
                ));
            }
            PublicKeyFilter::Selected(key)
        }
    };

    let order_by = match extract_url_param(&request, "order")?.as_deref() {
        None | Some("score") => FancyOrderBy::Score,
        Some("created") => FancyOrderBy::Created,
        Some(_) => return Err(bad_request("order must be score or created")),
    };
    let order_ascending = match extract_url_param(&request, "dir")?.as_deref() {
        None | Some("desc") => false,
        Some("asc") => true,
        Some(_) => return Err(bad_request("dir must be asc or desc")),
    };

    let limit = extract_url_int_param(&request, "limit")?
        .unwrap_or(50)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = extract_url_int_param(&request, "offset")?
        .unwrap_or(0)
        .clamp(0, MAX_OFFSET);
    let since = extract_url_date_param(&request, "since")?;
    let provider_id = extract_url_param(&request, "provider_id")?;

    let params = FancySearchParams {
        pattern,
        pattern_mode: mode,
        category,
        order_by,
        order_ascending,
        reserved,
        since,
        provider_id,
        public_key_base,
        limit,
        offset,
    };

    let pool = get_pool(&server_data).await;
    let result = match fancy_search(&pool, &params).await {
        Ok(result) => result,
        Err(e) => {
            log::error!("Error searching fancy addresses: {}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    Ok(HttpResponse::Ok().json(FancySearchResponse {
        entries: result.entries,
        total: result.total,
        total_capped: result.total_capped,
        limit,
        offset,
    }))
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct FancyStatsResponse {
    total: i64,
    free: i64,
    newest: Option<NaiveDateTime>,
    estimated_work_th: f64,
    categories: Vec<FancyCategoryStatDbObj>,
}

struct StatsCache {
    computed_at: Instant,
    response: FancyStatsResponse,
}

lazy_static! {
    static ref STATS_CACHE: Mutex<Option<StatsCache>> = Mutex::new(None);
}

const STATS_CACHE_TTL: Duration = Duration::from_secs(60);

pub async fn handle_stats(
    server_data: web::Data<Box<ServerData>>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut cache = STATS_CACHE.lock().await;
    if let Some(cached) = cache.as_ref() {
        if cached.computed_at.elapsed() < STATS_CACHE_TTL {
            return Ok(HttpResponse::Ok().json(&cached.response));
        }
    }

    let pool = get_pool(&server_data).await;
    let categories = match fancy_stats(&pool).await {
        Ok(categories) => categories,
        Err(e) => {
            log::error!("Error computing fancy stats: {}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };
    let work_events = match fancy_estimated_work_events(&pool).await {
        Ok(events) => events,
        Err(e) => {
            log::error!("Error computing estimated work: {}", e);
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    let response = FancyStatsResponse {
        total: categories.iter().map(|c| c.total).sum(),
        free: categories.iter().map(|c| c.free).sum(),
        newest: categories.iter().filter_map(|c| c.newest).max(),
        // Same semantics as /api/fancy/total_hash: each leading_zeroes entry
        // above 1e11 difficulty counts as one 1e11-hash event.
        estimated_work_th: work_events as f64 * 1e11 / 1e12,
        categories,
    };
    *cache = Some(StatsCache {
        computed_at: Instant::now(),
        response: response.clone(),
    });
    Ok(HttpResponse::Ok().json(&response))
}
