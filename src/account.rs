use tokio::fs;
use actix_web::{get, http, web, Responder, HttpResponse};
use actix_identity::Identity;

use crate::error::Result;
use crate::web::ServerData;
use crate::template;

#[get("/account/me.html")]
pub async fn me<'a>(identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    if identity.identity().is_some() {
        let mut body = fs::read_to_string("public/account/me.html").await?;
        template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
        Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(body))
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}
