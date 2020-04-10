use std::collections::HashMap;

use actix_http::HttpMessage;
use actix_web::{get, http, web, HttpRequest, HttpResponse, Responder};
use tokio::{fs, task::JoinHandle};
use tokio_postgres::{self as psql, NoTls};
use actix_identity::Identity;
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::error::Result;
use crate::i18n::Language;
use crate::template;

pub struct ServerData<'a> {
    pub(crate) client: psql::Client,
    pub(crate) argon: argon2::Config<'a>,
    pub(crate) lang: HashMap<String, Language>,
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
        let mut langs = HashMap::new();
        let l10n = client.query("select code, path from l10n", &[]).await?;
        for row in l10n {
            let key = row.get::<_, &str>("code").to_string();
            let path = row.get::<_, &str>("path");
            let text = fs::read_to_string(path).await?;
            let lang: Language = ron::de::from_str(&text)?;
            langs.insert(key, lang);
        }
        Ok(Self {
            client,
            argon: argon2::Config::default(),
            lang: langs,
            _handle: handle,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WordData {
    which: String,
}

#[get("/style/{sheet}.css")]
pub async fn stylesheet<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/style/{}.css", info);
    let sheet = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/css")
        .body(sheet))
}

#[get("/frontend/{script}.js")]
pub async fn javascript<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.js", info);
    let script = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/javascript")
        .body(script))
}

#[get("/{script}.wasm")]
pub async fn wasm<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.wasm", info);
    let script = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/wasm")
        .body(script))
}

#[get("/articles/{article}")]
pub async fn articles<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    let path = "public/articles/template.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace_recursive(
        &identity,
        &data.client,
        &data.lang[&lang],
        &mut body,
        &[format!("articles/{}", info)],
    )
    .await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/api/whoami")]
pub async fn api_whoami<'a>(
    _req: HttpRequest,
    identity: Identity,
    _data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let body = json!({
        "username": identity.identity().unwrap_or_else(String::new)
    });
    let body = body.to_string();
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}

#[get("/api/l10n")]
pub async fn api_l10n<'a>(
    req: HttpRequest,
    _identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    let body = serde_json::to_string(&data.lang[&lang])?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}

#[get("/api/t9n")]
pub async fn api_t9n<'a>(
    word: web::Query<WordData>,
    req: HttpRequest,
    _identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    let body = json!({
        "t9n": &data.lang[&lang][&word.which]
    });
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}

#[get("/{res}.html")]
pub async fn index<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    let path = format!("public/{}.html", info);
    let mut body = fs::read_to_string(path).await?;
    template::search_replace_recursive(&identity, &data.client, &data.lang[&lang], &mut body, &[])
        .await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/")]
pub async fn root<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    let path = "public/index.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace_recursive(&identity, &data.client, &data.lang[&lang], &mut body, &[])
        .await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}
