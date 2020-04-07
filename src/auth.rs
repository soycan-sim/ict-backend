use tokio::fs;
use actix_web::{get, post, web, Responder, HttpResponse};
use actix_identity::Identity;
use tokio_postgres as psql;
use serde::{Serialize, Deserialize};
use rand::prelude::*;

use crate::error::{Result, Error};
use crate::web::ServerData;
use crate::template;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthData {
    username: String,
    password: String,
}

pub fn salt() -> [u8; 32] {
    random()
}

#[post("/auth/create.html")]
pub async fn create<'a>(auth_data: web::Form<AuthData>, _identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    let salt = salt();
    let auth_data = auth_data.into_inner();
    let username = auth_data.username;
    let existing = data.client.query_opt("select * from users where username = $1", &[&username]).await?;
    if let Some(_existing) = existing {
        let mut body = fs::read_to_string("private/exists.html").await?;
        template::search_replace(&data.client, &mut body, &[format!("user {}", username)]).await?;
        return Ok(HttpResponse::BadRequest().body(body));
    }
    let pwhash = argon2::hash_encoded(auth_data.password.as_bytes(), &salt, &data.argon)?;
    data.client.execute("insert into users (username, pwhash) values ($1, $2)", &[&username, &pwhash]).await?;
    Ok(HttpResponse::SeeOther().header("Location", "/").finish())
}

#[post("/auth/login.html")]
pub async fn login<'a>(auth_data: web::Form<AuthData>, identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    let auth_data = auth_data.into_inner();
    let _userdata = query(&auth_data, &data).await?;
    let username = auth_data.username;
    identity.remember(username);
    Ok(HttpResponse::SeeOther().header("Location", "/").finish())
}

#[get("/auth/logout.html")]
pub async fn logout<'a>(identity: Identity, _data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    identity.forget();
    Ok(HttpResponse::SeeOther().header("Location", "/").finish())
}

async fn query<'a>(auth_data: &AuthData, data: &ServerData<'a>) -> Result<psql::Row> {
    let userdata = data.client.query_one("select * from users where username = $1", &[&auth_data.username]).await?;
    let pwhash = userdata.get::<_, &str>("pwhash");
    let res = argon2::verify_encoded(pwhash, auth_data.password.as_bytes())?;
    if res {
        Ok(userdata)
    } else {
        Err(Error::AuthenticationFailed)
    }
}
