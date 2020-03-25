#![feature(async_closure)]

use std::fmt::{self, Display};
use std::io::Error as IoError;
use std::num::ParseIntError;
use tokio::{fs, task::JoinHandle};
use tokio_postgres::{self as psql, NoTls, Error as DbError};
use actix_web::{get, web, http, ResponseError, App, HttpServer, Responder, HttpResponse};

pub mod template;
pub mod path;

#[derive(Debug)]
pub enum Error {
    Db(DbError),
    Io(IoError),
    Template(ParseIntError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Db(err) => Display::fmt(err, f),
            Error::Io(err) => Display::fmt(err, f),
            Error::Template(err) => write!(f, "template error: {}", err),
        }
    }
}

impl From<DbError> for Error {
    fn from(err: DbError) -> Error {
        Error::Db(err)
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::Template(err)
    }
}

impl ResponseError for Error { }

type Result<T> = std::result::Result<T, Error>;

struct ServerData {
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
async fn stylesheet(_data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/style/{}.css", info);
    let sheet = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/css")
        .body(sheet))
}

#[get("/frontend/{script}.js")]
async fn javascript(_data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.js", info);
    let script = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/javascript")
        .body(script))
}

#[get("/frontend/{script}.wasm")]
async fn wasm(_data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.wasm", info);
    let script = fs::read_to_string(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/wasm")
        .body(script))
}

#[get("/articles/{article}")]
async fn articles(data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = "public/articles/template.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace(&data.client, &mut body, &[&format!("articles/{}", info)]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/{res}.html")]
async fn index(data: web::Data<ServerData>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/{}.html", info);
    let mut body = fs::read_to_string(path).await?;
    template::search_replace(&data.client, &mut body, &[]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[get("/")]
async fn root(data: web::Data<ServerData>) -> Result<impl Responder> {
    let path = "public/index.html";
    let mut body = fs::read_to_string(path).await?;
    template::search_replace(&data.client, &mut body, &[]).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(body))
}

#[actix_rt::main]
async fn main() -> Result<()> {
    let data = || ServerData::new("host=localhost port=5432 dbname=testdb user=postgres", NoTls);
    HttpServer::new(move || App::new()
                    .data_factory(data)
                    .service(root)
                    .service(index)
                    .service(articles)
                    .service(stylesheet)
                    .service(javascript)
                    .service(wasm))
        .bind("127.0.0.1:8080")?
        .run()
        .await
        .map_err(From::from)
}
