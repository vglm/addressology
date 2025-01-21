use crate::api::contract::compile::handle_compile;
use crate::api::fancy::{
    handle_fancy_buy_api, handle_fancy_deploy_start, handle_fancy_estimate_total_hash,
    handle_fancy_new, handle_list, handle_list_best_score, handle_list_newest, handle_random,
};
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
    .route("/fancy/random",                 get().to(handle_random))
    .route("/fancy/list",                   get().to(handle_list))
    .route("/fancy/list_newest",            get().to(handle_list_newest))
    .route("/fancy/total_hash",             get().to(handle_fancy_estimate_total_hash))
    .route("/fancy/list_best_score",        get().to(handle_list_best_score))
    .route("/fancy/new",                    post().to(handle_fancy_new))
    .route("/fancy/buy/{address}",          post().to(handle_fancy_buy_api))
    .route("/fancy/deploy/{contract_id}",   post().to(handle_fancy_deploy_start))
    .route("/contract/compile",             post().to(handle_compile))
    .route("/greet",                        get().to(handle_greet))
    .route("/contract/{contract_id}",       get().to(contract::get_contract_info_api))
    .route("/contract/new",                 post().to(contract::insert_contract_info_api))
    .route("/contract/{contract_id}",       post().to(contract::update_contract_info_api))
    .route("/contracts/list",               get().to(contract::get_contracts_api))
    .route("contract/{contract_id}/delete", post().to(contract::delete_contract_api))
}
