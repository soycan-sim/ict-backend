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
pub struct CreateData {
    firstname: String,
    lastname: String,
    username: String,
    email: String,
    password: String,
    password2: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthData {
    username: String,
    password: String,
}

pub fn salt() -> [u8; 32] {
    random()
}

#[post("/auth/create.html")]
pub async fn create<'a>(auth_data: web::Form<CreateData>, _identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    let salt = salt();
    let auth_data = auth_data.into_inner();
    let firstname = if auth_data.firstname.is_empty() {
        None
    } else {
        Some(auth_data.firstname)
    };
    let lastname = if auth_data.lastname.is_empty() {
        None
    } else {
        Some(auth_data.lastname)
    };
    if auth_data.username.is_empty() {
        return Err(Error::InvalidCreateUser("username is empty".to_string()));
    }
    if auth_data.password.is_empty() {
        return Err(Error::InvalidCreateUser("password is empty".to_string()));
    }
    let username = auth_data.username;
    let email = auth_data.email;
    let existing = data.client.query_opt("select * from users where username = $1", &[&username]).await?;
    if let Some(_existing) = existing {
        let mut body = fs::read_to_string("private/exists.html").await?;
        template::search_replace(&data.client, &mut body, &[format!("user {}", username)]).await?;
        return Ok(HttpResponse::BadRequest().body(body));
    }
    if !email.contains('@') {
        return Err(Error::InvalidCreateUser("e-mail is not an e-mail".to_string()));
    }
    if auth_data.password != auth_data.password2 {
        return Err(Error::PasswordMismatch);
    }
    let pwhash = argon2::hash_encoded(auth_data.password.as_bytes(), &salt, &data.argon)?;
    match (firstname, lastname) {
        (Some(first), Some(last)) => {
            data.client.execute("insert into users (firstname, lastname, username, email, pwhash) values ($1, $2, $3, $4, $5)", &[&first, &last, &username, &email, &pwhash]).await?;
        }
        (Some(first), None) => {
            data.client.execute("insert into users (firstname, username, email, pwhash) values ($1, $2, $3, $4)", &[&first, &username, &email, &pwhash]).await?;
        }
        (None, Some(last)) => {
            data.client.execute("insert into users (lastname, username, email, pwhash) values ($1, $2, $3, $4)", &[&last, &username, &email, &pwhash]).await?;
        }
        (None, None) => {
            data.client.execute("insert into users (username, email, pwhash) values ($1, $2, $3)", &[&username, &email, &pwhash]).await?;
        }
    }
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
