#![feature(async_closure)]

use clap::{App as Clapp, SubCommand};
use tokio_postgres::NoTls;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpServer};
use actix_web::FromRequest;
use arrayvec::ArrayString;
use actix_web_middleware_redirect_https::RedirectHTTPS;

use crate::error::{Error, Result};

pub mod account;
pub mod auth;
pub mod error;
pub mod i18n;
pub mod path;
pub mod template;
pub mod term;
pub mod web;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

fn psql_escape<S: AsRef<str>>(string: S) -> String {
    string.as_ref().replace("\\", "\\\\")
}

fn psql_config(password: &str) -> String {
    format!(
        "host=localhost port=5432 dbname=ict user=ict password='{}'",
        psql_escape(password)
    )
}

#[actix_rt::main]
async fn main() -> Result<()> {
    let matches = Clapp::new("ict-backend")
        .version(VERSION)
        .author(AUTHORS)
        .subcommand(SubCommand::with_name("start").about(
            "starts the ict webservice in the current directory (must \
                    be ran as `ict`)",
        ))
        .get_matches();

    match matches.subcommand() {
        ("start", Some(_matches)) => {
            let window = pancurses::initscr();
            let password = term::prompt(&window, Some("Password: "), true);
            pancurses::endwin();
            let password = password.unwrap_or_else(String::new);
            let password = ArrayString::<[_; 255]>::from(&password).unwrap_or_else(|err| {
                panic!(
                    "The password is too long, length is {}, but maximum length is 255",
                    err.element().len()
                );
            });
            let data = move || web::ServerData::new(psql_config(&password), NoTls);
            let mut server_config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
            let mut file = std::io::BufReader::new(std::fs::File::open("example.com_ssl_certificate.cer")?);
            let certs = rustls::internal::pemfile::certs(&mut file).unwrap();
            let mut file = std::io::BufReader::new(std::fs::File::open("example.com_private_key.key")?);
            let mut private_keys = rustls::internal::pemfile::rsa_private_keys(&mut file).unwrap();
            let private_key = private_keys.pop().unwrap();
            server_config.set_single_cert(certs, private_key).unwrap();
            HttpServer::new(move || {
                App::new()
                    .data_factory(data)
                    .wrap(IdentityService::new(
                        CookieIdentityPolicy::new(&[0; 64])
                            .name("auth-cookie")
                            .secure(true),
                    ))
                    .app_data(
                        actix_web::web::Json::<web::ClientResource>::configure(|cfg| {
                            cfg.limit((1 << 31) - 1)
                        })
                    )
                    .wrap(RedirectHTTPS::default())
                    .service(auth::create)
                    .service(auth::login)
                    .service(auth::logout)
                    .service(auth::change_email)
                    .service(auth::change_password)
                    .service(account::me)
                    .service(account::admin_panel)
                    .service(account::api_setadmin)
                    .service(account::api_setemployee)
                    .service(account::editor)
                    .service(account::draft)
                    .service(account::new)
                    .service(account::save)
                    .service(account::api_draft)
                    .service(account::wasm)
                    .service(i18n::lang)
                    .service(web::api_whoami)
                    .service(web::api_l10n)
                    .service(web::api_t9n)
                    .service(web::api_neworder)
                    .service(web::api_endorder)
                    .service(web::api_addclass)
                    .service(web::root)
                    .service(web::index)
                    .service(web::articles)
                    .service(web::stylesheet)
                    .service(web::javascript)
                    .service(web::wasm)
                    .service(web::static_jpg)
                    .service(web::static_png)
                    .service(web::static_svg)
                    .service(web::static_pdf)
                    .service(web::static_mp4)
                    .service(web::resource)
            })
            // .bind("127.0.0.1:8080")?
            .bind("0.0.0.0:80")?
            .bind_rustls("0.0.0.0:443", server_config)?
            .run()
            .await
            .map_err(From::from)
        }
        ("", _) => Err(Error::Cmdline("no command passed".to_string())),
        (x, _) => Err(Error::Cmdline(format!("unrecognized command: {:?}", x))),
    }
}
