use tokio::{fs, task::JoinHandle};
use tokio_postgres::{self as psql, NoTls};
use actix_web::{get, web, http, Responder, HttpResponse};

use crate::error::Result;
use crate::template;

pub struct ServerData {
    client: psql::Client,
    _handle: JoinHandle<()>,
}

impl ServerData {
    pub async fn new(config: &str, tls: NoTls) -> Result<Self> {
        let (client, conn) = psql::connect(config, tls).await?;
        let handle = tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("connection error: {}", e);
            }
        });
        Ok(Self { client, _handle: handle })
    }
}

#[get("/style/{sheet}.css")]
pub async fn stylesheet(_data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/style/{}.css", info);
    let sheet = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/css")
        .body(sheet))
}

#[get("/frontend/{script}.js")]
pub async fn javascript(_data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.js", info);
    let script = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/javascript")
        .body(script))
}

#[get("/frontend/{script}.wasm")]
pub async fn wasm(_data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.wasm", info);
    let script = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/wasm")
        .body(script))
}

#[get("/articles/{article}")]
pub async fn articles(data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = "public/articles/template.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace(&data.client, &mut body, &[format!("articles/{}", info)]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/{res}.html")]
pub async fn index(data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/{}.html", info);
    let mut body = fs::read_to_string(path).await?;
    template::search_replace(&data.client, &mut body, &[]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/")]
pub async fn root(data: web::Data<ServerData>) -> Result<impl Responder> {
    let path = "public/index.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace(&data.client, &mut body, &[]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}
