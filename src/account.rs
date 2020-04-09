use std::path::Path;

use tokio::fs;
use actix_web::{get, http, post, web, Responder, HttpResponse};
use actix_identity::Identity;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use crate::web::ServerData;
use crate::template;

#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleData {
    title: String,
    article: String,
}

fn pathify(string: &str) -> (String, String) {
    let public_path = format!("articles/{}", string.replace(|ch: char| !ch.is_alphanumeric(), "-"));
    let private_path = format!("public/{}", &public_path);
    (public_path, private_path)
}

fn draftify(user: &str, string: &str) -> String {
    let public_path = format!("drafts/{}", string.replace(|ch: char| !ch.is_alphanumeric(), "-"));
    let private_path = format!("private/{}/{}", user, &public_path);
    private_path
}

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

#[post("/account/draft.html")]
pub async fn save<'a>(auth_data: web::Form<ArticleData>, identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let uid = data.client.query_one(
            "select users.id as uid from users where username = $1",
            &[&username]
        ).await?;
        let uid = uid.get::<_, i32>("uid");
        let user = data.client.query_opt(
            "select employees.id from employees where employees.uid = $1",
            &[&uid]
        ).await?;
        if user.is_some() {
            let auth_data = auth_data.into_inner();
            let title = auth_data.title;
            let article = auth_data.article;
            let mut private = draftify(&username, &title);
            private.push_str(".md");
            let existing = data.client.query("select * from drafts where path = $2 and title != $1", &[&title, &private]).await?;
            if !existing.is_empty() {
                let mut body = fs::read_to_string("private/exists.html").await?;
                template::search_replace_recursive(&identity, &data.client, &mut body, &[format!("article {}", title)]).await?;
                return Ok(HttpResponse::BadRequest().body(body));
            }
            let path: &Path = private.as_ref();
            let directory = path.parent().expect("`draftify()` didn't return a proper path");
            fs::create_dir_all(directory).await?;
            fs::write(&private, article).await?;
            data.client.execute("insert into drafts (path, title, author) values ($1, $2, $3)", &[&private, &title, &uid]).await?;
            Ok(HttpResponse::Ok().finish())
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[post("/account/editor.html")]
pub async fn new<'a>(auth_data: web::Form<ArticleData>, identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let uid = data.client.query_one(
            "select users.id as uid from users where username = $1",
            &[&username]
        ).await?;
        let uid = uid.get::<_, i32>("uid");
        let user = data.client.query_opt(
            "select employees.id from employees where employees.uid = $1",
            &[&uid]
        ).await?;
        if user.is_some() {
            let auth_data = auth_data.into_inner();
            let title = auth_data.title;
            let article = auth_data.article;
            let (mut public, mut private) = pathify(&title);
            public.push_str(".md");
            private.push_str(".md");
            let existing = data.client.query("select * from articles where title = $1 or path = $2", &[&title, &private]).await?;
            if !existing.is_empty() {
                let mut body = fs::read_to_string("private/exists.html").await?;
                template::search_replace_recursive(&identity, &data.client, &mut body, &[format!("article {}", title)]).await?;
                return Ok(HttpResponse::BadRequest().body(body));
            }
            fs::write(&private, article).await?;
            data.client.execute("insert into articles (path, title, cdate, author) values ($1, $2, current_date, $3)", &[&public, &title, &uid]).await?;
            Ok(HttpResponse::SeeOther().header("Location", format!("/{}", public)).finish())
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/editor.html")]
pub async fn editor<'a>(identity: Identity, data: web::Data<ServerData<'a>>) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let user = data.client.query_opt(
            "select employees.id from employees where employees.uid = \
             (select users.id as uid from users where username = $1)",
            &[&username]
        ).await?;
        if user.is_some() {
            let mut body = fs::read_to_string("public/account/editor.html").await?;
            template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(body))
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/draft/{draft}.md")]
pub async fn draft<'a>(identity: Identity, data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let uid = data.client.query_one(
            "select users.id as uid from users where username = $1",
            &[&username]
        ).await?;
        let uid = uid.get::<_, i32>("uid");
        let user = data.client.query_opt(
            "select employees.id from employees where employees.uid = $1",
            &[&uid]
        ).await?;
        if user.is_some() {
            let path = format!("private/{}/drafts/{}.md", username, info);
            let existing = data.client.query_opt("select title from drafts where author = $1 and path = $2", &[&uid, &path]).await?;
            if let Some(existing) = existing {
                let mut body = fs::read_to_string("public/account/editor.html").await?;
                let content = fs::read_to_string(path).await?;
                let title = existing.get::<_, Option<&str>>("title");
                let args = if let Some(title) = title {
                    vec![content, title.to_string()]
                } else {
                    vec![content]
                };
                template::search_replace_recursive(&identity, &data.client, &mut body, &args).await?;
                Ok(HttpResponse::Ok()
                    .header(http::header::CONTENT_TYPE, "text/html")
                    .body(body))
            } else {
                Ok(HttpResponse::SeeOther().header("Location", "/account/editor.html".to_string()).finish())
            }
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(&identity, &data.client, &mut body, &[]).await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/{script}.wasm")]
pub async fn wasm<'a>(_identity: Identity, _data: web::Data<ServerData<'a>>, info: web::Path<String>) -> Result<impl Responder> {
    let path = format!("public/frontend/{}.wasm", info);
    let script = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/wasm")
        .body(script))
}
