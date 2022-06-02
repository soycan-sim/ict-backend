use std::path::Path;

use actix_http::HttpMessage;
use actix_web::{get, http, post, web, HttpRequest, HttpResponse, Responder};
use tokio::fs;
use chrono::offset::TimeZone;
use actix_identity::Identity;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::Result;
use crate::template;
use crate::web::ServerData;

#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleData {
    title: String,
    article: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveDeleteData {
    title: String,
    article: String,
    delete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiDraftData {
    id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetAdminData {
    value: bool,
    uid: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetEmployeeData {
    value: bool,
    uid: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddClassData {
    title: String,
    teacher: i32,
    price: f32,
    edate: String,
    etime: String,
    elength: String,
    maxcount: Option<i32>,
    language: String,
    description: String,
    image: String,
    hyperlink: String,
}

fn pathify(string: &str) -> (String, String) {
    let public_path = format!(
        "articles/{}",
        string.replace(|ch: char| !ch.is_alphanumeric(), "-")
    );
    let private_path = format!("public/{}", &public_path);
    (public_path, private_path)
}

fn draftify(user: &str, string: &str) -> String {
    let public_path = format!(
        "drafts/{}",
        string.replace(|ch: char| !ch.is_alphanumeric(), "-")
    );
    let private_path = format!("private/{}/{}", user, &public_path);
    private_path
}

#[get("/account/me.html")]
pub async fn me<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if identity.identity().is_some() {
        let mut body = fs::read_to_string("public/account/me.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(body))
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/admin.html")]
pub async fn admin_panel<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if let Some(username) = identity.identity() {
        let admin = data.client.query_opt(
            "select id from admins where uid = \
             (select id as uid from users where username = $1)",
            &[&username]
        ).await?;
        if admin.is_none() {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(
                &identity,
                &data.client,
                &data.lang[&lang],
                &mut body,
                &[],
            )
            .await?;
            return Ok(HttpResponse::Forbidden().body(body));
        }

        let mut body = fs::read_to_string("public/account/admin.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(body))
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[post("/account/draft.html")]
pub async fn save<'a>(
    draft_data: web::Json<SaveDeleteData>,
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if let Some(username) = identity.identity() {
        let uid = data
            .client
            .query_one(
                "select users.id as uid from users where username = $1",
                &[&username],
            )
            .await?;
        let uid = uid.get::<_, i32>("uid");
        let user = data
            .client
            .query_opt(
                "select employees.id from employees where employees.uid = $1",
                &[&uid],
            )
            .await?;
        if user.is_some() {
            let draft_data = draft_data.into_inner();
            let title = draft_data.title;
            let article = draft_data.article;
            if draft_data.delete {
                let mut private = draftify(&username, &title);
                private.push_str(".md");
                data
                    .client
                    .execute(
                        "delete from drafts where path = $2 and title = $1 and author = $3",
                        &[&title, &private, &uid],
                    )
                    .await?;
            } else {
                let mut private = draftify(&username, &title);
                private.push_str(".md");
                let existing = data
                    .client
                    .query(
                        "select * from drafts where path = $2 and title != $1 and author = $3",
                        &[&title, &private, &uid],
                    )
                    .await?;
                if !existing.is_empty() {
                    let mut body = fs::read_to_string("private/exists.html").await?;
                    template::search_replace_recursive(
                        &identity,
                        &data.client,
                        &data.lang[&lang],
                        &mut body,
                        &[format!("article {}", title)],
                    )
                    .await?;
                    return Ok(HttpResponse::BadRequest().body(body));
                }
                let path: &Path = private.as_ref();
                let directory = path
                    .parent()
                    .expect("`draftify()` didn't return a proper path");
                fs::create_dir_all(directory).await?;
                fs::write(&private, article).await?;
                let existing = data
                    .client
                    .query_opt(
                        "select id from drafts where path = $1",
                        &[&private],
                    )
                    .await?;
                if let Some(row) = existing {
                    let id = row.get::<_, i32>("id");
                    data.client
                        .execute(
                            "update drafts set title = $1 where id = $2",
                            &[&title, &id],
                        )
                        .await?;
                } else {
                    data.client
                        .execute(
                            "insert into drafts (path, title, author) values ($1, $2, $3)",
                            &[&private, &title, &uid],
                        )
                        .await?;
                }
            }
            Ok(HttpResponse::Ok().finish())
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(
                &identity,
                &data.client,
                &data.lang[&lang],
                &mut body,
                &[],
            )
            .await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[post("/api/setadmin")]
pub async fn api_setadmin<'a>(
    admin_data: web::Form<SetAdminData>,
    _req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let admin = data.client.query_opt(
            "select id from admins where uid = \
             (select id as uid from users where username = $1)",
            &[&username]
        ).await?;
        if admin.is_none() {
            let body = json!({
                "success": false,
                "reason": "forbidden"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::Forbidden()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        }

        if admin_data.value {
            let existing = data
                .client
                .query_opt("select id from admins where uid = $1", &[&admin_data.uid])
                .await?;
            if existing.is_some() {
                let body = json!({
                    "success": false,
                    "reason": "bad request"
                });
                let body = serde_json::to_string(&body).unwrap();

                return Ok(HttpResponse::BadRequest()
                   .header(http::header::CONTENT_TYPE, "application/json")
                   .body(body));
            }
            data
                .client
                .execute("insert into admins (uid) values ($1)", &[&admin_data.uid])
                .await?;
        } else {
            data
                .client
                .execute("delete from admins where uid = $1", &[&admin_data.uid])
                .await?;
        }

        let body = json!({
            "success": true
        });
        let body = serde_json::to_string(&body).unwrap();

        Ok(HttpResponse::Ok()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body))
    } else {
        let body = json!({
            "success": false,
            "reason": "forbidden"
        });
        let body = serde_json::to_string(&body).unwrap();

        Ok(HttpResponse::Forbidden()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body))
    }
}

#[post("/api/setemployee")]
pub async fn api_setemployee<'a>(
    employee_data: web::Form<SetEmployeeData>,
    _req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let admin = data.client.query_opt(
            "select id from admins where uid = \
             (select id as uid from users where username = $1)",
            &[&username]
        ).await?;
        if admin.is_none() {
            let body = json!({
                "success": false,
                "reason": "forbidden"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::Forbidden()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        }

        if employee_data.value {
            let existing = data
                .client
                .query_opt("select id from employees where uid = $1", &[&employee_data.uid])
                .await?;
            if existing.is_some() {
                let body = json!({
                    "success": false,
                    "reason": "bad request"
                });
                let body = serde_json::to_string(&body).unwrap();

                return Ok(HttpResponse::BadRequest()
                   .header(http::header::CONTENT_TYPE, "application/json")
                   .body(body));
            }
            data
                .client
                .execute("insert into employees (uid) values ($1)", &[&employee_data.uid])
                .await?;
        } else {
            data
                .client
                .execute("delete from employees where uid = $1", &[&employee_data.uid])
                .await?;
        }

        let body = json!({
            "success": true
        });
        let body = serde_json::to_string(&body).unwrap();

        Ok(HttpResponse::Ok()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body))
    } else {
        let body = json!({
            "success": false,
            "reason": "forbidden"
        });
        let body = serde_json::to_string(&body).unwrap();

        Ok(HttpResponse::Forbidden()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body))
    }
}

#[post("/api/addclass")]
pub async fn api_addclass<'a>(
    class_data: web::Form<AddClassData>,
    _req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    if let Some(username) = identity.identity() {
        let admin = data.client.query_opt(
            "select id from admins where uid = \
             (select id as uid from users where username = $1)",
            &[&username]
        ).await?;
        if admin.is_none() {
            let body = json!({
                "success": false,
                "reason": "forbidden"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::Forbidden()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        }

        let teacher = data
            .client
            .query_opt(
                "select id from employees where employees.uid = $1",
                &[&class_data.teacher],
            )
            .await?;

        if teacher.is_none() {
            let body = json!({
                "success": false,
                "reason": "forbidden"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::Forbidden()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        }

        let date = if let Ok(date) = chrono::NaiveDate::parse_from_str(&class_data.edate, "%Y-%m-%d") {
            date
        } else {
            let body = json!({
                "success": false,
                "reason": "bad-request"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::BadRequest()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        };
        let time = if let Ok(time) = chrono::NaiveTime::parse_from_str(&class_data.etime, "%H:%M") {
            time
        } else {
            let body = json!({
                "success": false,
                "reason": "bad-request"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::BadRequest()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        };
        let duration = if let Ok(duration) = chrono::NaiveTime::parse_from_str(&class_data.elength, "%H:%M") {
            duration
        } else {
            let body = json!({
                "success": false,
                "reason": "bad-request"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::BadRequest()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        };
        // NOTE: hardcoded UTC+1
        const HOUR: i32 = 3600;
        let datetime = chrono::offset::FixedOffset::from_offset(
                &chrono::offset::FixedOffset::east(1 * HOUR)
            )
            .from_utc_date(&date)
            .and_time(time);
        let datetime = if let Some(datetime) = datetime {
            datetime
        } else {
            let body = json!({
                "success": false,
                "reason": "bad-request"
            });
            let body = serde_json::to_string(&body).unwrap();

            return Ok(HttpResponse::BadRequest()
               .header(http::header::CONTENT_TYPE, "application/json")
               .body(body));
        };

        data
            .client
            .execute(
                "insert into classes \
                        (title, teacher, price, cdate, etime, \
                         elength, count, maxcount, language) \
                     values \
                        ($1, $2, $3::float4::numeric::money, current_date, $4, $5::text::interval, $6, $7, $8)",
                &[
                    &class_data.title,
                    &class_data.teacher,
                    &class_data.price,
                    &datetime,
                    &duration.format("%H:%M").to_string(),
                    &0_i32,
                    &class_data.maxcount.unwrap_or(65535),
                    &"de",
                ],
            )
            .await?;

        // let body = json!({
        //     "success": true
        // });
        // let body = serde_json::to_string(&body).unwrap();

        // Ok(HttpResponse::Ok()
        //    .header(http::header::CONTENT_TYPE, "application/json")
        //    .body(body))
        Ok(HttpResponse::SeeOther()
            .header("Location", "/schedule.html")
            .finish())
    } else {
        let body = json!({
            "success": false,
            "reason": "forbidden"
        });
        let body = serde_json::to_string(&body).unwrap();

        Ok(HttpResponse::Forbidden()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body))
    }
}

#[get("/api/draft")]
pub async fn api_draft<'a>(
    draft_data: web::Query<ApiDraftData>,
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if let Some(username) = identity.identity() {
        let uid = data
            .client
            .query_one(
                "select users.id as uid from users where username = $1",
                &[&username],
            )
            .await?;
        let uid = uid.get::<_, i32>("uid");
        let article = data
            .client
            .query_one(
                "select path, title from drafts where id = $1 and author = $2",
                &[&draft_data.id, &uid],
                )
            .await?;
        let content = fs::read_to_string(article.get::<_, &str>("path")).await?;
        let title = article.get::<_, &str>("title");
        let body = json!({
            "content": content,
            "title": title
        });
        let body = serde_json::to_string(&body)?;
        Ok(HttpResponse::Ok()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body))
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[post("/account/editor.html")]
pub async fn new<'a>(
    auth_data: web::Form<ArticleData>,
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if let Some(username) = identity.identity() {
        let uid = data
            .client
            .query_one(
                "select users.id as uid from users where username = $1",
                &[&username],
            )
            .await?;
        let uid = uid.get::<_, i32>("uid");
        let user = data
            .client
            .query_opt(
                "select employees.id from employees where employees.uid = $1",
                &[&uid],
            )
            .await?;
        if user.is_some() {
            let auth_data = auth_data.into_inner();
            let title = auth_data.title;
            let article = auth_data.article;
            let (mut public, mut private) = pathify(&title);
            public.push_str(".md");
            private.push_str(".md");
            let existing = data
                .client
                .query(
                    "select * from articles where title = $1 or path = $2",
                    &[&title, &private],
                )
                .await?;
            if !existing.is_empty() {
                let mut body = fs::read_to_string("private/exists.html").await?;
                template::search_replace_recursive(
                    &identity,
                    &data.client,
                    &data.lang[&lang],
                    &mut body,
                    &[format!("article {}", title)],
                )
                .await?;
                return Ok(HttpResponse::BadRequest().body(body));
            }
            fs::write(&private, article).await?;
            data.client.execute("insert into articles (path, title, cdate, author) values ($1, $2, current_date, $3)", &[&public, &title, &uid]).await?;

            let mut draft_path = draftify(&username, &title);
            draft_path.push_str(".md");
            data
                .client
                .execute(
                    "delete from drafts where title = $1 and path = $2 and author = $3",
                    &[&title, &draft_path, &uid],
                )
                .await?;

            Ok(HttpResponse::SeeOther()
                .header("Location", format!("/{}", public))
                .finish())
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(
                &identity,
                &data.client,
                &data.lang[&lang],
                &mut body,
                &[],
            )
            .await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/editor.html")]
pub async fn editor<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if let Some(username) = identity.identity() {
        let user = data
            .client
            .query_opt(
                "select employees.id from employees where employees.uid = \
             (select users.id as uid from users where username = $1)",
                &[&username],
            )
            .await?;
        if user.is_some() {
            let mut body = fs::read_to_string("public/account/editor.html").await?;
            template::search_replace_recursive(
                &identity,
                &data.client,
                &data.lang[&lang],
                &mut body,
                &[],
            )
            .await?;
            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(body))
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(
                &identity,
                &data.client,
                &data.lang[&lang],
                &mut body,
                &[],
            )
            .await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/draft/{draft}.md")]
pub async fn draft<'a>(
    req: HttpRequest,
    identity: Identity,
    data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    if let Some(username) = identity.identity() {
        let uid = data
            .client
            .query_one(
                "select users.id as uid from users where username = $1",
                &[&username],
            )
            .await?;
        let uid = uid.get::<_, i32>("uid");
        let user = data
            .client
            .query_opt(
                "select employees.id from employees where employees.uid = $1",
                &[&uid],
            )
            .await?;
        if user.is_some() {
            let path = format!("private/{}/drafts/{}.md", username, info);
            let existing = data
                .client
                .query_opt(
                    "select title from drafts where author = $1 and path = $2",
                    &[&uid, &path],
                )
                .await?;
            if let Some(existing) = existing {
                let mut body = fs::read_to_string("public/account/editor.html").await?;
                let content = fs::read_to_string(path).await?;
                let title = existing.get::<_, Option<&str>>("title");
                let args = if let Some(title) = title {
                    vec![content, title.to_string()]
                } else {
                    vec![content]
                };
                template::search_replace_recursive(
                    &identity,
                    &data.client,
                    &data.lang[&lang],
                    &mut body,
                    &args,
                )
                .await?;
                Ok(HttpResponse::Ok()
                    .header(http::header::CONTENT_TYPE, "text/html")
                    .body(body))
            } else {
                Ok(HttpResponse::SeeOther()
                    .header("Location", "/account/editor.html".to_string())
                    .finish())
            }
        } else {
            let mut body = fs::read_to_string("private/forbidden.html").await?;
            template::search_replace_recursive(
                &identity,
                &data.client,
                &data.lang[&lang],
                &mut body,
                &[],
            )
            .await?;
            Ok(HttpResponse::Forbidden().body(body))
        }
    } else {
        let mut body = fs::read_to_string("private/forbidden.html").await?;
        template::search_replace_recursive(
            &identity,
            &data.client,
            &data.lang[&lang],
            &mut body,
            &[],
        )
        .await?;
        Ok(HttpResponse::Forbidden().body(body))
    }
}

#[get("/account/{script}.wasm")]
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
