use crate::db::model::UserDbObj;
use crate::db::ops::get_user;
use crate::{login_check_and_get, ServerData};
use actix_session::Session;
use actix_web::web::Data;
use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct UserTokensResp {
    uid: String,
    email: String,
    tokens: i64,
}

pub async fn handle_get_user_tokens(data: Data<Box<ServerData>>, session: Session) -> HttpResponse {
    let session_user: UserDbObj = login_check_and_get!(session);

    let conn = data.db_connection.lock().await;
    let user = match get_user(&*conn, &session_user.email).await {
        Ok(user) => user,
        Err(err) => {
            log::error!("Error getting user: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };
    HttpResponse::Ok().json(UserTokensResp {
        uid: user.uid,
        email: user.email,
        tokens: user.tokens,
    })
}
