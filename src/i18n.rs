use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Index;

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use actix_identity::Identity;
use actix_web::cookie::Cookie;
use actix_web::{get, http, web, HttpRequest, HttpResponse, Responder};

use crate::error::Result;
use crate::web::ServerData;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Language {
    code: String,
    language: String,
    t9n: HashMap<String, String>,
}

impl Language {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn language(&self) -> &str {
        &self.language
    }
}

impl<'a, S> Index<&'a S> for Language
where
    S: Eq + Hash + Debug,
    String: Borrow<S>,
{
    type Output = str;

    fn index(&self, key: &'a S) -> &Self::Output {
        self.t9n.get(key).unwrap_or_else(|| panic!("no l10n found for key {:?}", key))
    }
}

#[get("/lang/{lang}.html")]
pub async fn lang<'a>(
    _req: HttpRequest,
    _identity: Identity,
    data: web::Data<ServerData<'a>>,
    info: web::Path<String>,
) -> Result<impl Responder> {
    let lang = info.to_string();
    if !data.lang.contains_key(&lang) {
        return Ok(HttpResponse::BadRequest()
            .finish());
    }

    let mut cookie = Cookie::new("lang", lang);
    cookie.set_path("/");
    cookie.make_permanent();
    Ok(HttpResponse::SeeOther()
        .cookie(cookie)
        .header(http::header::LOCATION, "/")
        .finish())
}
