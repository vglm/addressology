mod api;
mod config;
mod cookie;
mod db;
mod deploy;
mod email;
mod error;
mod fancy;
mod hash;
mod oauth;
mod solc;
mod types;
mod update;

use crate::api::scope::server_api_scope;
use crate::config::get_base_difficulty_price;
use crate::cookie::load_key_or_create;
use crate::db::connection::create_sqlite_connection;
use crate::db::model::DeployStatus;
use crate::db::ops::{
    fancy_list_all, fancy_update_score, get_all_contracts_by_deploy_status_and_network,
    insert_fancy_obj,
};
use crate::deploy::handle_fancy_deploy;
use crate::fancy::parse_fancy;
use crate::fancy::score_fancy;
use crate::hash::compute_create3_command;
use crate::types::DbAddress;
use actix_multipart::form::MultipartFormConfig;
use actix_multipart::MultipartError;
use actix_session::config::CookieContentSecurity;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::SameSite;
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder};
use awc::Client;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

fn get_allowed_emails() -> Vec<String> {
    let res = env::var("ALLOWED_EMAILS")
        .unwrap_or("sieciech.czajka@golem.network".to_string())
        .split(",")
        .map(|x| x.trim().to_string())
        .collect();
    log::info!("Allowed emails loaded: {:?}", res);
    res
}

fn get_domain() -> String {
    let res = env::var("WEB_PORTAL_DOMAIN").unwrap_or("localhost".to_string());

    log::info!("Portal domain: {}", res);

    res
}

lazy_static! {
    pub static ref ALLOWED_EMAILS: Vec<String> = get_allowed_emails();
    pub static ref WEB_PORTAL_DOMAIN: String = get_domain();
    static ref PASS_SALT: String = env::var("PASS_SALT").unwrap_or("LykwVQJAcU".to_string());
    pub static ref ALLOW_CREATING_NEW_ACCOUNTS: bool = env::var("ALLOW_CREATING_NEW_ACCOUNTS")
        .map(|v| v == "true")
        .unwrap_or(false);
}

pub struct ServerData {
    pub db_connection: Arc<Mutex<SqlitePool>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeployData {
    pub address: DbAddress,
    pub network: String,
    pub bytecode: String,
    pub constructor_args: String,
}

#[cfg(feature = "dashboard")]
#[derive(rust_embed::RustEmbed)]
#[folder = "frontend/dist"]
struct Asset;

pub async fn redirect_to_dashboard() -> impl Responder {
    {
        let target = "/dashboard/";
        log::debug!("Redirecting to endpoint: {target}");
        HttpResponse::Ok()
            .status(actix_web::http::StatusCode::PERMANENT_REDIRECT)
            .append_header((actix_web::http::header::LOCATION, target))
            .finish()
    }
}

#[allow(dead_code)]
async fn proxy(
    path: web::Path<String>,
    client: web::Data<Client>,
    request: HttpRequest,
) -> HttpResponse {
    log::info!("Proxying request to: {path}");
    let url = format!("http://localhost:5173/dashboard/{path}");

    // here we use `IntoHttpResponse` to return the request to
    // duckduckgo back to the client that called this endpoint

    let mut new_request = client.request(request.method().clone(), url);
    for (header_name, header_value) in request.headers() {
        new_request = new_request.insert_header((header_name.clone(), header_value.clone()));
    }
    match new_request.send().await {
        Ok(resp) => {
            log::info!("Response: {}", resp.status());
            let mut response = HttpResponse::build(resp.status());

            resp.headers().into_iter().for_each(|(k, v)| {
                response.insert_header((k, v));
            });

            response.streaming(resp)
        }
        Err(e) => {
            log::error!("Error: {e}");
            HttpResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR).body(format!("Error: {e}"))
        }
    }
}
#[allow(clippy::needless_return)]
#[allow(unreachable_code)]
pub async fn dashboard_serve(
    path: web::Path<String>,
    _client: web::Data<Client>,
    _request: HttpRequest,
) -> HttpResponse {
    #[cfg(feature = "dashboard")]
    {
        let mut path = path.as_str();

        let mut compression_header = None;
        let compressions_to_check = vec![".br", ".gz", ""];

        let mut content = None;
        if path.ends_with(".gz") || path.ends_with(".br") {
            content = Asset::get(path);
        } else {
            for compression in compressions_to_check {
                let path_with_compress = format!("{}{}", path, compression);
                content = Asset::get(&path_with_compress);
                if content.is_some() {
                    if compression == ".br" {
                        compression_header = Some("br");
                    } else if compression == ".gz" {
                        compression_header = Some("gzip");
                    }
                    break;
                }
            }
            if content.is_none() && !path.contains('.') {
                path = "index.html";
                content = Asset::get(path);
            }
        }

        log::debug!("Serving frontend file: {path}");

        return match content {
            Some(content) => {
                let mut builder: HttpResponseBuilder = HttpResponseBuilder::new(StatusCode::OK);
                builder.content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref());
                if let Some(compression) = compression_header {
                    builder.append_header(("Content-Encoding", compression));
                }
                builder.append_header(("Cache-Control", "public, max-age=3600")); // 1 hour
                builder.body(content.data.into_owned())
            }
            None => HttpResponse::NotFound().body("404 Not Found"),
        };
    }
    #[cfg(feature = "proxy")]
    {
        return proxy(path, _client, _request).await;
    }
    #[cfg(all(not(feature = "dashboard"), not(feature = "proxy")))]
    HttpResponse::NotFound().body(format!("404 Not Found: {}", path))
}

/// Enum that defines the available subcommands
#[derive(Subcommand)]
enum Commands {
    Test {},
    ScoreFancy {},
    ProcessDeploy {
        #[arg(short, long)]
        network: String,
    },
    ComputeCreate3 {
        #[arg(short, long)]
        factory: String,
        #[arg(short, long)]
        salt: String,
    },
    AddFancyAddress {
        #[arg(short, long)]
        factory: String,
        #[arg(short, long)]
        salt: String,
    },
    /// Start web server
    Server {
        #[arg(long, default_value = "localhost:80")]
        addr: String,

        #[arg(long)]
        threads: Option<usize>,
    },
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(long, default_value = "addressology.sqlite")]
    db: String,
}

fn handle_multipart_error(err: MultipartError, _req: &HttpRequest) -> actix_web::Error {
    log::error!("Multipart error: {}", err);
    actix_web::Error::from(err)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG").unwrap_or("info".to_string()),
    );
    env_logger::init();

    let args = Cli::parse();

    let secret_key = load_key_or_create("web-portal-cookie.key");

    match args.cmd {
        Commands::Server { addr, threads } => {
            let conn = create_sqlite_connection(Some(&PathBuf::from(args.db)), None, false, true)
                .await
                .unwrap();

            HttpServer::new(move || {
                let cors = actix_cors::Cors::permissive();

                let server_data = web::Data::new(Box::new(ServerData {
                    db_connection: Arc::new(Mutex::new(conn.clone())),
                }));
                let client = web::Data::new(Client::new());
                let session_middleware =
                    SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                        .cookie_secure(true)
                        .cookie_content_security(CookieContentSecurity::Private)
                        .cookie_same_site(SameSite::Strict)
                        .cookie_domain(Some(WEB_PORTAL_DOMAIN.to_string()))
                        .cookie_name("web-portal-session".to_string())
                        .build();

                App::new()
                    .wrap(session_middleware)
                    .wrap(cors)
                    .app_data(server_data)
                    .app_data(client)
                    .app_data(
                        MultipartFormConfig::default()
                            .total_limit(10 * 1024 * 1024 * 1024) // 10 GB
                            .memory_limit(10 * 1024 * 1024) // 10 MB
                            .error_handler(handle_multipart_error),
                    )
                    .route("/", web::get().to(redirect_to_dashboard))
                    .route("/dashboard", web::get().to(redirect_to_dashboard))
                    .route("/dashboard/{_:.*}", web::get().to(dashboard_serve))
                    .route("/service/update", web::post().to(update::push_update))
                    .service(server_api_scope())
            })
            .workers(threads.unwrap_or(std::thread::available_parallelism().unwrap().into()))
            .bind(addr)?
            .run()
            .await
        }
        Commands::ScoreFancy {} => {
            let conn = create_sqlite_connection(Some(&PathBuf::from(args.db)), None, false, true)
                .await
                .unwrap();

            let fancies = fancy_list_all(&conn).await.unwrap();

            for fancy in fancies {
                let score = score_fancy(fancy.address.addr());
                log::info!(
                    "Fancy: {:#x} Score: {}",
                    fancy.address.addr(),
                    score.total_score
                );

                let new_price =
                    (score.price_multiplier * get_base_difficulty_price() as f64) as i64;
                if fancy.score != score.total_score
                    || fancy.price != new_price
                    || fancy.category != score.category
                {
                    log::info!("Updating score for: {:#x}", fancy.address.addr());
                    match fancy_update_score(
                        &conn,
                        fancy.address,
                        score.total_score,
                        new_price,
                        &score.category,
                    )
                    .await
                    {
                        Ok(_) => (),
                        Err(e) => {
                            log::error!("{}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            Ok(())
        }
        Commands::ProcessDeploy { network } => {
            let conn = create_sqlite_connection(Some(&PathBuf::from(args.db)), None, false, true)
                .await
                .unwrap();

            let contracts = get_all_contracts_by_deploy_status_and_network(
                &conn,
                DeployStatus::Requested,
                network,
            )
            .await
            .unwrap();

            if let Some(contract) = contracts.first() {
                log::info!("Processing contract: {:#?}", contract);

                match handle_fancy_deploy(&conn, contract.clone()).await {
                    Ok(_) => {
                        log::info!("Deployment successful");
                        Ok(())
                    }
                    Err(e) => {
                        log::error!("{}", e);
                        std::process::exit(1)
                    }
                }
            } else {
                log::info!("No contracts to process");
                Ok(())
            }
        }
        Commands::ComputeCreate3 { factory, salt } => {
            let result = compute_create3_command(&factory, &salt);
            match result {
                Ok(hash) => {
                    log::info!("Computed create3 hash: {}", hash);
                    println!("{}", hash);
                }
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            }
            Ok(())
        }
        Commands::AddFancyAddress { factory, salt } => {
            let conn = create_sqlite_connection(Some(&PathBuf::from(args.db)), None, false, true)
                .await
                .unwrap();

            let factory = web3::types::Address::from_str(&factory).unwrap();
            let result = match parse_fancy(salt, factory) {
                Ok(fancy) => fancy,
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            };
            let mut db_trans = match conn.begin().await {
                Ok(db) => db,
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            };

            match insert_fancy_obj(&mut *db_trans, result).await {
                Ok(_) => (),
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            }

            match db_trans.commit().await {
                Ok(_) => (),
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            }

            Ok(())
        }
        Commands::Test {} => {
            //test_command(conn).await;
            /*match compile_solc(
                "// SPDX-License-Identifier: UNLICENSED\npragma solidity ^0.8.28;\n\n// Uncomment this line to use console.log\n// import \"hardhat/console.sol\";\n\ncontract Lock {\n    uint public unlockTime;\n    address payable public owner;\n\n    event Withdrawal(uint amount, uint when);\n\n    constructor(uint _unlockTime) payable {\n        require(\n            block.timestamp < _unlockTime,\n            \"Unlock time should be in the future\"\n        );\n\n        unlockTime = _unlockTime;\n        owner = payable(msg.sender);\n    }\n\n    function withdraw() public {\n        // Uncomment this line, and the import of \"hardhat/console.sol\", to print a log in your terminal\n        // console.log(\"Unlock time is %o and block timestamp is %o\", unlockTime, block.timestamp);\n\n        require(block.timestamp >= unlockTime, \"You can't withdraw yet\");\n        require(msg.sender == owner, \"You aren't the owner\");\n\n        emit Withdrawal(address(this).balance, block.timestamp);\n\n        owner.transfer(address(this).balance);\n    }\n}\n\n",
                               "0.8.28").await {
                Ok(res) => (
                    println!("Output of compilation: {:#?}", res)
                    ),
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            }*/

            Ok(())
        }
    }
}
