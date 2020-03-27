#![feature(async_closure)]

use std::process;
use std::fmt::{self, Display};
use std::io::Error as IoError;
use std::num::ParseIntError;
use clap::{Arg, App as Clapp, SubCommand, ArgMatches};
use tokio::{fs, task::JoinHandle};
use tokio_postgres::{self as psql, NoTls, Error as DbError};
use actix_web::{get, web, http, ResponseError, App, HttpServer, Responder, HttpResponse};

pub mod template;
pub mod path;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = "circus-backend is an open source webservice framework";
const AFTER_HELP: &str = "This program was made possible by https://Zirkus-Internationale.de.";

#[derive(Debug)]
pub enum Error {
    Db(DbError),
    Io(IoError),
    Template(ParseIntError),
    Cmdline(String),
    Useradd,
    CreateDb,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Db(err) => Display::fmt(err, f),
            Error::Io(err) => Display::fmt(err, f),
            Error::Template(err) => write!(f, "template error: {}", err),
            Error::Cmdline(err) => write!(f, "command line error: {}", err),
            Error::Useradd => write!(f, "creating the user `circus` failed (`useradd ... circus`)"),
            Error::CreateDb => write!(f, "creating the database `circus` failed (`createdb ... circus`)"),
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

fn init_user<'a, 'b>(_matches: &'a ArgMatches<'b>) -> Result<()> {
    let mut child = process::Command::new("useradd")
        .arg("-m")
        .arg("-d")
        .arg("/var/code-circus")
        .arg("-U")
        .arg("-G")
        .arg("tty,lp,disk,wheel,floppy,audio,cdrom,dialout,video,cdrw,usb,input,plugdev")
        .arg("circus")
        .spawn()?;
    let status = child.wait()?;
    if !status.success() {
        return Err(Error::Useradd)
    }

    let mut child = process::Command::new("passwd")
        .arg("circus")
        .spawn()?;
    let status = child.wait()?;
    if !status.success() {
        return Err(Error::Useradd)
    }

    let mut child = process::Command::new("sudo")
        .arg("-u")
        .arg("mkdir")
        .arg("/var/code-circus/public")
        .spawn()?;
    let status = child.wait()?;
    if !status.success() {
        return Err(Error::Useradd)
    }

    let mut child = process::Command::new("sudo")
        .arg("-u")
        .arg("circus")
        .arg("git")
        .arg("init")
        .arg("/var/code-circus/public")
        .spawn()?;
    let status = child.wait()?;
    if !status.success() {
        return Err(Error::Useradd)
    }

    let mut child = process::Command::new("passwd")
        .arg("circus")
        .spawn()?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Useradd)
    }
}

fn init_db<'a, 'b>(_matches: &'a ArgMatches<'b>) -> Result<()> {
    let mut child = process::Command::new("createuser")
        .arg("-d")
        .arg("-r")
        .arg("circus")
        .spawn()?;
    let status = child.wait()?;
    if !status.success() {
        return Err(Error::CreateDb);
    }

    let mut child = process::Command::new("createdb")
        .arg("-O")
        .arg("circus")
        .arg("-U")
        .arg("circus")
        .arg("-W")
        .arg("circus")
        .arg("the default and main CODE_Circus database")
        .spawn()?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::CreateDb)
    }
}

fn git_add<'a, 'b>(matches: &'a ArgMatches<'b>) -> Result<()> {
    let mut child = process::Command::new("git")
        .arg("add")
        .args(matches.values_of_lossy("file").into_iter().flatten())
        .spawn()?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::CreateDb)
    }
}

fn git_commit<'a, 'b>(matches: &'a ArgMatches<'b>) -> Result<()> {
    let mut child = process::Command::new("git")
        .arg("commit")
        .args(matches.value_of_lossy("message").as_ref().map(AsRef::as_ref))
        .spawn()?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::CreateDb)
    }
}

#[actix_rt::main]
async fn main() -> Result<()> {
    let matches = Clapp::new("circus-backend")
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .after_help(AFTER_HELP)
        .subcommand(SubCommand::with_name("init-db")
            .about("initializes the circus database with the circus user (must \
                    be ran as `postgres`)"))
        .subcommand(SubCommand::with_name("init-user")
            .about("initializes the circus user (must be ran as `root`)"))
        .subcommand(SubCommand::with_name("add")
            .about("adds the files specified in the command line to the staging \
                    area in the current working directory")
            .arg(Arg::with_name("file")
                .takes_value(true)
                .required(true)
                .multiple(true)
                .allow_hyphen_values(true)
                .value_name("FILE")))
        .subcommand(SubCommand::with_name("commit")
            .about("commits the files in the staging area in the current working \
                    directory to /var/circus-www")
            .arg(Arg::with_name("message")
                .short("m")
                .long("message")
                .takes_value(true)
                .allow_hyphen_values(true)
                .value_name("MESSAGE")))
        .subcommand(SubCommand::with_name("start")
            .about("starts the circus webservice in the current directory (must \
                    be ran as `circus`)"))
        .get_matches();

    match matches.subcommand() {
        ("init-db", Some(matches)) => init_db(matches),
        ("init-user", Some(matches)) => init_user(matches),
        ("add", Some(matches)) => git_add(matches),
        ("commit", Some(matches)) => git_commit(matches),
        ("start", Some(_matches)) => {
            let data = || ServerData::new("host=localhost port=5432 dbname=circus user=circus", NoTls);
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
        ("", _) => Err(Error::Cmdline("no command passed".to_string())),
        (x, _) => Err(Error::Cmdline(format!("unrecognized command: {:?}", x))),
    }
}
