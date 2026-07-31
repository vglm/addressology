use crate::api::contract::compile::handle_compile;
use crate::api::fancy::buy::handle_fancy_buy_api;
use crate::api::fancy::deploy::handle_fancy_deploy_start;
use crate::api::fancy::estimate::handle_fancy_estimate_total_hash;
use crate::api::fancy::job::{handle_finish_job, handle_job_list, handle_new_job};
use crate::api::fancy::list::handle_list;
use crate::api::fancy::my::handle_my_list;
use crate::api::fancy::new::handle_fancy_new_many;
use crate::api::fancy::score::{handle_get_score_categories, handle_score_custom};
use crate::api::fancy::search::{handle_search, handle_stats};
use crate::api::fancy::tokens::handle_get_user_tokens;
use crate::api::fancy::{handle_public_key_list, handle_random};
use crate::api::oauth::google::{handle_google_callback, handle_login_via_google};
use crate::api::user::handle_greet;
use crate::api::{contract, user};
use actix_web::web::{get, post};
use actix_web::Scope;

#[rustfmt::skip]
pub fn server_api_scope() -> Scope {
    Scope::new("/api")
    .route("/auth/callback/google",         get().to(handle_google_callback))
    .route("/auth/login/google",            get().to(handle_login_via_google))
    .route("/login",                        post().to(user::handle_login))
    .route("/session/check",                get().to(user::handle_session_check))
    .route("/is_login",                     get().to(user::handle_is_login))
    .route("/is_login",                     post().to(user::handle_is_login))
    .route("/logout",                       post().to(user::handle_logout))
    .route("/reset_pass",                   post().to(user::handle_password_reset))
    .route("/set_pass",                     post().to(user::handle_password_set))
    .route("/change_pass",                  post().to(user::handle_password_change))
    .route("/user/tokens",                  get().to(handle_get_user_tokens))
    .route("/fancy/score/{address}",        get().to(handle_score_custom))
    .route("/fancy/categories",             get().to(handle_get_score_categories))
    .route("/fancy/random",                 get().to(handle_random))
    .route("/fancy/total_hash",             get().to(handle_fancy_estimate_total_hash))
    .route("/fancy/list",                   get().to(handle_list))
    .route("/fancy/search",                 get().to(handle_search))
    .route("/fancy/stats",                  get().to(handle_stats))
    .route("/fancy/mylist",                 get().to(handle_my_list))
    .route("/fancy/new_many",               post().to(handle_fancy_new_many))
    .route("/fancy/new_many2",              post().to(handle_fancy_new_many))
    .route("/fancy/buy/{address}",          post().to(handle_fancy_buy_api))
    .route("/fancy/deploy/{contract_id}",   post().to(handle_fancy_deploy_start))
    .route("/public_key_base/list",         get().to(handle_public_key_list))
    .route("/job/new",                      post().to(handle_new_job))
    .route("/job/finish/{job_id}",          post().to(handle_finish_job))
    .route("/job/list",                     get().to(handle_job_list))
    .route("/contract/compile",             post().to(handle_compile))
    .route("/greet",                        get().to(handle_greet))
    .route("/contract/{contract_id}",       get().to(contract::get_contract_info_api))
    .route("/contract/new",                 post().to(contract::insert_contract_info_api))
    .route("/contract/{contract_id}",       post().to(contract::update_contract_info_api))
    .route("/contracts/list",               get().to(contract::get_contracts_api))
    .route("/contracts/assignments",        get().to(contract::get_all_contract_assignments))
    .route("contract/{contract_id}/delete", post().to(contract::delete_contract_api))
}
