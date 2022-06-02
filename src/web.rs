use std::collections::HashMap;

use actix_http::HttpMessage;
use actix_web::{get, http, web, post, HttpRequest, HttpResponse, Responder};
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, Transport};
use tokio::{fs, task::JoinHandle};
use tokio_postgres::{self as psql, NoTls};
use actix_identity::Identity;
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::error::Result;
use crate::i18n::Language;
use crate::template;

const SMTP_SERVER: &str = "smtp.example.com";
const PAYPAL_OAUTH_API: &str = "https://api-m.paypal.com/v1/oauth2/token/";
const PAYPAL_ORDER_API: &str = "https://api-m.paypal.com/v2/checkout/orders/";

#[derive(Debug, Deserialize)]
struct Auth {
    access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct NewOrderData {
    class_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct Name {
    given_name: String,
    surname: String,
}

#[derive(Debug, Deserialize)]
pub struct Payer {
    email_address: String,
    name: Name,
    payer_id: String,
}

#[derive(Debug, Deserialize)]
pub struct EndOrderData {
    id: String,
    intent: String,
    payer: Payer,
}

#[derive(Debug, Deserialize)]
struct NewOrder {
    error: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EndOrder {
    error: Option<String>,
    id: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddClassData {
    title: String,
    language: String,
    teacher: i32,
    description: String,
    image: String,
    price: String,
    edate: String,
    etime: String,
    elength: String,
    maxcount: i32,
    hyperlink: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientResource {
    filename: String,
    mime: String,
    data: String,
}

pub struct ServerData<'a> {
    pub(crate) client: psql::Client,
    pub(crate) argon: argon2::Config<'a>,
    pub(crate) lang: HashMap<String, Language>,
    pub(crate) http: actix_web::client::Client,
    paypal_auth: Auth,
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
        let http = actix_web::client::Client::default();
        let client_id = fs::read_to_string("./client").await?;
        let secret = fs::read_to_string("./secret").await?;
        let auth = http.post(PAYPAL_OAUTH_API)
            .header("Accept", "application/json")
            .basic_auth(client_id, Some(&secret))
            .send_body("grant_type=client_credentials")
            .await?
            .json::<Auth>()
            .await?;
        Ok(Self {
            client,
            argon: argon2::Config::default(),
            lang: langs,
            http,
            paypal_auth: auth,
            _handle: handle,
        })
    }
}

fn rand_string() -> String {
    const LEN: usize = 32;
    let mut string = String::new();
    for _ in 0..LEN {
        let num = rand::random::<u8>() % 62;
        if num < 26 {
            string.push((num + b'a') as char);
        } else if num < 52 {
            string.push((num - 26 + b'A') as char);
        } else {
            string.push((num - 52 + b'0') as char);
        }
    }
    string
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



#[post("/api/neworder")]
pub async fn api_neworder<'a>(
    info: web::Json<NewOrderData>,
    _req: HttpRequest,
    _identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let class_id = info.class_id;
    let class = data.client.query_one(
        "select price::numeric::text as price, title from classes where id = $1",
        &[&class_id],
    ).await?;
    let price = class.get::<_, &str>("price");
    let class_title = class.get::<_, &str>("title");
    let order = data.http.post(PAYPAL_ORDER_API)
        .header("Accept", "application/json")
        .bearer_auth(&data.paypal_auth.access_token)
        .send_json(&json!({
            "intent": "CAPTURE",
            "purchase_units": [{
                "amount": {
                    "currency_code": "EUR",
                    "value": price,
                },
                "description": class_title
            }],
        }))
        .await?
        .json::<NewOrder>()
        .await?;
    if let Some(error) = order.error {
        eprintln!("{}", error);
        return Ok(HttpResponse::InternalServerError()
           .finish());
    }
    let orderid = order.id.unwrap();
    data.client.execute(
        "insert into orders (class, count, orderid, finished) values ($1, 1, $2, false)",
        &[&class_id, &orderid]
    ).await?;
    let body = json!({
        "orderid": orderid,
    });
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}

#[post("/api/endorder")]
pub async fn api_endorder<'a>(
    info: web::Json<EndOrderData>,
    req: HttpRequest,
    _identity: Identity,
    data: web::Data<ServerData<'a>>,
) -> Result<impl Responder> {
    let order = data.http.get(&format!("{}{}", PAYPAL_ORDER_API, info.id))
            .header("Accept", "application/json")
            .bearer_auth(&data.paypal_auth.access_token)
            .send()
            .await?
            .json::<EndOrder>()
            .await?;
    if let Some(error) = order.error {
        eprintln!("{}", error);
        return Ok(HttpResponse::InternalServerError()
           .finish());
    }
    data.client.execute(
        "update orders set firstname = $1, lastname = $2, email = $3",
        &[&info.payer.name.given_name, &info.payer.name.surname, &info.payer.email_address],
    ).await?;
    let prev_order = data.client.query_one(
        "select class, finished from orders where orderid = $1",
        &[&order.id],
    ).await?;
    
    if prev_order.get::<_, bool>("finished") {
        let body = json!({
            "ok": false,
            "error": "this order has been already finished"
        });
        return Ok(HttpResponse::BadRequest()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body));
    }

    data.client.execute(
        "update orders set finished = true where orderid = $1",
        &[&order.id],
    ).await?;

    let class_id = prev_order.get::<_, i32>("class");
    let class = data.client.query_one(
        "select etime, hyperlink from classes where id = $1",
        &[&class_id],
    ).await?;

    let datetime = class.get::<_, chrono::DateTime<chrono::offset::Local>>("etime");
    let date = datetime.format("%d-%m-%Y");
    let time = datetime.format("%H:%M");
    let hyperlink = class.get::<_, &str>("hyperlink");
    
    let us = "user@example.com";
    let cc = "user@gmail.com";
    let name = format!("{} {}", info.payer.name.given_name, info.payer.name.surname);
    let them = &info.payer.email_address;
    
    let lang = req
        .cookie("lang")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "de".to_string());
    let subject = &data.lang[&lang]["confirmation_subject"];
    let body_hi = &data.lang[&lang]["confirmation_body_hi"];
    let body_main_0 = &data.lang[&lang]["confirmation_body_main_0"];
    let body_main_1 = &data.lang[&lang]["confirmation_body_main_1"];
    let body_main_2 = &data.lang[&lang]["confirmation_body_main_2"];
    let body_bye = &data.lang[&lang]["confirmation_body_bye"];

    let envelope = lettre::Envelope::new(Some(us.parse().unwrap()), vec![them.parse().unwrap(), cc.parse().unwrap()]).unwrap();
    let message = format!("To: {}\nSubject: {}\nFrom: {}\nCc: {}\n\n{} {},\n\n{} {} {} {}{}\n\n{}\n\n{}", them, subject, us, cc, body_hi, name, body_main_0, date, body_main_1, time, body_main_2, hyperlink, body_bye).into_bytes();
    let email = lettre::SendableEmail::new(envelope, "".to_string(), message);

    let smtp_user = fs::read_to_string("smtp-username").await?;
    let smtp_pass = fs::read_to_string("smtp-password").await?;
    let creds = Credentials::new(smtp_user, smtp_pass);

    let mut smtp = SmtpClient::new_simple(SMTP_SERVER)
        .unwrap()
        .smtp_utf8(true)
        .credentials(creds)
        .transport();

    // prayge that nothing happens
    smtp.send(email).unwrap();
    
    let body = json!({
        "ok": true,
    });
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}

#[post("/api/addclass")]
pub async fn api_addclass<'a>(
    info: web::Form<AddClassData>,
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
        return Ok(HttpResponse::Forbidden().body(body));
    }

    data.client.execute("insert into classes (title,
                                              teacher,
                                              price,
                                              cdate,
                                              etime,
                                              elength,
                                              count,
                                              maxcount,
                                              language,
                                              description,
                                              image,
                                              hyperlink)
                             values ($1, $2, $3::text::numeric::money, current_date,
                                       ($4::text || ' ' ||  $5::text)::timestamptz,
                                     $6::text::interval, 0, $7, $8, $9, $10, $11)",
                        &[&info.title, &info.teacher, &info.price, &info.edate, &info.etime,
                          &info.elength, &info.maxcount, &info.language, &info.description, &info.image, &info.hyperlink],
    ).await?;

    let body = json!({
        "ok": true,
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

#[get("/static/{res}.pdf")]
pub async fn static_pdf<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/static/{}.pdf", info);
    let body = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "application/pdf")
        .body(body))
}

#[get("/static/{res}.mp4")]
pub async fn static_mp4<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/static/{}.mp4", info);
    let body = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "video/mp4")
        .body(body))
}

#[get("/static/{res}.jpg")]
pub async fn static_jpg<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/static/{}.jpg", info);
    let body = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "image/jpeg")
        .body(body))
}

#[get("/static/{res}.png")]
pub async fn static_png<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/static/{}.png", info);
    let body = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "image/png")
        .body(body))
}

#[get("/static/{res}.svg")]
pub async fn static_svg<'a>(
    _req: HttpRequest,
    _identity: Identity,
    _data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let path = format!("public/static/{}.svg", info);
    let body = fs::read(path).await?;
    Ok(HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "image/svg+xml")
        .body(body))
}

#[post("/api/resource")]
pub async fn resource<'a>(
    info: web::Json<ClientResource>,
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
        return Ok(HttpResponse::Forbidden().body(body));
    }

    let format = match info.mime.as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    };

    if format.is_none() {
        let body = json!({
            "ok": false,
            "error": format!("refusing to save mime type {}", info.mime),
        });
        return Ok(HttpResponse::BadRequest()
           .header(http::header::CONTENT_TYPE, "application/json")
           .body(body));
    }

    let data = base64::decode(&info.data);
    let data = match data {
        Ok(data) => data,
        Err(err) => {
            let body = json!({
                "ok": false,
                "error": format!("invalid base64 data: {}", err),
            });
            return Ok(HttpResponse::BadRequest()
                      .header(http::header::CONTENT_TYPE, "application/json")
                      .body(body));
        }
    };

    let filename = rand_string();
    let server_filename = format!("public/static/{}.{}", filename, format.unwrap());
    let client_filename = format!("/static/{}.{}", filename, format.unwrap());

    fs::write(&server_filename, &data).await?;
    
    let body = json!({
        "ok": true,
        "filename": client_filename,
    });
    Ok(HttpResponse::Ok()
              .header(http::header::CONTENT_TYPE, "application/json")
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
