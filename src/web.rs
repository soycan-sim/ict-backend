use tokio::{fs, task::JoinHandle};
use tokio_postgres::{self as psql, NoTls};
use actix_web::{get, web, http, Responder, HttpResponse};
use actix_identity::Identity;
use serde_json::json;

use crate::error::Result;
use crate::template;

pub struct ServerData<'a> {
    pub(crate) client: psql::Client,
    pub(crate) argon: argon2::Config<'a>,
    _handle: JoinHandle<()>,
}

impl ServerData<'static> {
    pub async fn new<S: Into<String>>(config: S, tls: NoTls) -> Result<Self> {
        let (client, conn) = psql::connect(&config.into(), tls).await?;
        let handle = tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("connection error: {}", e);
            }
        });
        Ok(Self { client, argon: argon2::Config::default(), _handle: handle })
    }
}

#[get("/style/{sheet}.css")]
pub async fn stylesheet<'a>(_identity: Identity, _data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/style/{}.css", info);
    let sheet = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/css")
        .body(sheet))
}

#[get("/frontend/{script}.js")]
pub async fn javascript<'a>(_identity: Identity, _data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.js", info);
    let script = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/javascript")
        .body(script))
}

#[get("/{script}.wasm")]
pub async fn wasm<'a>(_identity: Identity, _data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.wasm", info);
    let script = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/wasm")
        .body(script))
}

#[get("/articles/{article}")]
pub async fn articles<'a>(identity: Identity, data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    let path = "public/articles/template.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace_recursive(&identity, &data.client, &mut body, &[format!("articles/{}", info)]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/api/whoami")]
pub async fn whoami<'a>(identity: Identity, _data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    let body = json!({
        "username": identity.identity().unwrap_or_else(String::new)
    });
    let body = body.to_string();
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}

#[get("/{res}.html")]
pub async fn index<'a>(identity: Identity, data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/{}.html", info);
    let mut body = fs::read_to_string(path).await?;
    template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/")]
pub async fn root<'a>(identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    let path = "public/index.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}
