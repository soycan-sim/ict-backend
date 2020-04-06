#![feature(async_closure)]

use std::process;
use clap::{Arg, App as Clapp, SubCommand, ArgMatches};
use tokio_postgres::NoTls;
use actix_web::{App, HttpServer};
use actix_identity::{CookieIdentityPolicy, IdentityService};

use crate::error::{Error, Result};

pub mod auth;
pub mod error;
pub mod template;
pub mod path;
pub mod web;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = "circus-backend is an open source webservice framework";
const AFTER_HELP: &str = "This program was made possible by https://Zirkus-Internationale.de.";

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
            let data = || web::ServerData::new("host=localhost port=5432 dbname=circus user=circus", NoTls);
            HttpServer::new(move || App::new()
                            .data_factory(data)
                            .wrap(IdentityService::new(
                                CookieIdentityPolicy::new(&[0; 64])
                                    .name("auth-cookie")
                                    .secure(false)))
                            .service(auth::create)
                            .service(auth::login)
                            .service(auth::logout)
                            .service(web::whoami)
                            .service(web::root)
                            .service(web::index)
                            .service(web::articles)
                            .service(web::stylesheet)
                            .service(web::javascript)
                            .service(web::wasm))
                .bind("127.0.0.1:8080")?
                .run()
                .await
                .map_err(From::from)
        }
        ("", _) => Err(Error::Cmdline("no command passed".to_string())),
        (x, _) => Err(Error::Cmdline(format!("unrecognized command: {:?}", x))),
    }
}
