use std::convert::TryFrom;
use std::str::FromStr;

use actix_identity::Identity;
use futures::future;
use futures::TryFutureExt;
use pulldown_cmark as md;
use tokio::fs;
use tokio_postgres as psql;

use crate::path::PublicPath;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
enum Pattern {
    Empty,
    Login,
    Editor,
    Me(String),
    Path(String),
    Positional(usize),
    ArticlePositional(usize),
    PreviewLatest(usize),
    ArticleLatest(usize),
    PreviewTitle(String),
    ArticleTitle(String),
    Maybe(Box<Pattern>),
}

async fn author(client: &psql::Client, uid: i32) -> Result<Option<String>> {
    let user = client.query_opt("select firstname, lastname, username from users where id = $1", &[&uid]).await?;
    match user {
        Some(user) => {
            let firstname = user.get::<_, Option<&str>>("firstname");
            let lastname = user.get::<_, Option<&str>>("lastname");
            let username = user.get::<_, &str>("username");
            match (firstname, lastname) {
                (Some(first), Some(last)) => {
                    Ok(Some(format!("{} \"{}\" {}", first, username, last)))
                }
                (Some(first), None) => {
                    Ok(Some(format!("{} \"{}\"", first, username)))
                }
                (None, Some(last)) => {
                    Ok(Some(format!("\"{}\" {}", username, last)))
                }
                _ => Ok(Some(username.to_string())),
            }
        }
        None => Ok(None),
    }
}

impl FromStr for Pattern {
    type Err = Error;

    fn from_str(pattern: &str) -> Result<Self> {
        if pattern.is_empty() {
            Ok(Pattern::Empty)
        } else if pattern == "login" {
            Ok(Pattern::Login)
        } else if pattern == "editor" {
            Ok(Pattern::Editor)
        } else if pattern.starts_with("me.") {
            Ok(Pattern::Me(pattern[3..].to_string()))
        } else if pattern.starts_with('/') {
            Ok(Pattern::Path(pattern[1..].to_string()))
        } else if pattern.starts_with('%') {
            Ok(Pattern::Positional(pattern[1..].parse()?))
        } else if pattern.starts_with("article%") {
            Ok(Pattern::ArticlePositional(pattern["article%".len()..].parse()?))
        } else if pattern.starts_with("preview~") {
            Ok(Pattern::PreviewLatest(pattern["preview~".len()..].parse()?))
        } else if pattern.starts_with("article~") {
            Ok(Pattern::ArticleLatest(pattern["article~".len()..].parse()?))
        } else if pattern.starts_with("preview ") {
            Ok(Pattern::PreviewTitle(pattern["preview ".len()..].to_string()))
        } else if pattern.starts_with("article ") {
            Ok(Pattern::ArticleTitle(pattern["article ".len()..].to_string()))
        } else if pattern.starts_with("maybe(") {
            let start = "maybe(".len();
            let end = pattern.len() - 1;
            if &pattern[end..] != ")" {
                return Err(Error::InvalidPattern(pattern.to_string()));
            }
            let sub = &pattern[start..end];
            Ok(Pattern::Maybe(Box::new(sub.parse()?)))
        } else {
            Err(Error::InvalidPattern(pattern.to_string()))
        }
    }
}

impl Pattern {
    pub async fn to_string_nonrecursive(
        self,
        identity: &Identity,
        client: &psql::Client,
        args: &[String],
    ) -> Result<String> {
        match self {
            Pattern::Empty => Ok(String::new()),
            Pattern::Login => {
                match identity.identity() {
                    Some(identity) => {
                        Ok(format!("<span class=\"float-right\"><a href=\"/auth/logout.html\">Logout</a></span> \
                                    <span class=\"float-right\"><a href=\"/account/me.html\">Logged in as: {}</a></span>", identity))
                    }
                    None => {
                        Ok("<span class=\"float-right\"><a href=\"/login.html\">Login</a></span> \
                            <span class=\"float-right\"><a href=\"/create.html\">Register</a></span>".to_string())
                    }
                }
            }
            Pattern::Editor => {
                match identity.identity() {
                    Some(identity) => {
                        // only employees are allowed to make new articles
                        let user = client.query_opt(
                            "select employees.id from employees where employees.uid = \
                             (select users.id as uid from users where username = $1)",
                            &[&identity]
                        ).await?;
                        if user.is_some() {
                            Ok("<span class=\"float-right\"><a href=\"/account/editor.html\">New</a></span>".to_string())
                        } else {
                            Err(Error::AuthorizationFailed)
                        }
                    }
                    None => {
                        Err(Error::AuthorizationFailed)
                    }
                }
            }
            Pattern::Me(field) => {
                if field == "pwhash" {
                    Ok("No passwords for you!".to_string())
                } else {
                    match identity.identity() {
                        Some(me) => {
                            match client.query_opt("select * from users where username = $1", &[&me]).await? {
                                Some(row) => Ok(row.get::<&str, &str>(&field).to_string()),
                                None => Ok("".to_string()),
                            }
                        }
                        None => Ok("".to_string()),
                    }
                }
            }
            Pattern::Path(path) => {
                let path = PublicPath::try_from(path)?;
                let text = fs::read_to_string(&path).await?;
                if path.extension() == Some("md".as_ref()) {
                    let parser = md::Parser::new_ext(&text, md::Options::all());
                    let mut html = String::new();
                    md::html::push_html(&mut html, parser);
                    Ok(html)
                } else {
                    Ok(text)
                }
            }
            Pattern::Positional(pos) => {
                let path = &args[pos - 1];
                let path = PublicPath::try_from(&**path)?;
                let text = fs::read_to_string(&path).await?;
                if path.extension() == Some("md".as_ref()) {
                    let parser = md::Parser::new_ext(&text, md::Options::all());
                    let mut html = String::new();
                    md::html::push_html(&mut html, parser);
                    Ok(html)
                } else {
                    Ok(text)
                }
            }
            Pattern::ArticlePositional(pos) => {
                let path = &args[pos - 1];
                let args: &[&(dyn psql::types::ToSql + Sync)] = &[path];
                let article = client
                    .query_one("select title, to_char(cdate, 'yyyy-mm-dd') as date, author from articles where path = $1", args);
                let contents = article
                    .map_err(From::from)
                    .and_then(async move |article| {
                        let path = PublicPath::try_from(&**path)?;
                        if path.exists() {
                            let text = fs::read_to_string(&path)
                                .await?;
                            if path.extension() == Some("md".as_ref()) {
                                let parser = md::Parser::new_ext(&text, md::Options::all());
                                let mut html = String::new();
                                md::html::push_html(&mut html, parser);
                                Ok((article, html))
                            } else {
                                Ok((article, text))
                            }
                        } else {
                            Err(Error::ResourceNotFound(path.to_string_lossy().to_string()))
                        }
                    });
                contents.and_then(async move |(article, contents)| {
                    let by_author = author(client, article.get::<_, i32>("author")).await?.map(|author| format!(" by {}", author)).unwrap_or_else(String::new);
                    Ok(format!(
                        "<article><h1>{}</h1>{}{}<br/>{}</article>",
                        article.get::<_, &str>("title"),
                        article.get::<_, &str>("date"),
                        by_author,
                        contents,
                    ))
                }).await
            }
            Pattern::PreviewLatest(no) => {
                let rows = client
                    .query("select title, path, to_char(cdate, 'yyyy-mm-dd') as date, author from articles order by cdate", &[])
                    .await?;
                let article = rows.len().checked_sub(no).and_then(|no| rows.get(no)).ok_or_else(|| Error::ResourceNotFound(format!("preview~{}", no)))?;
                let by_author = author(client, article.get::<_, i32>("author")).await?.map(|author| format!(" by {}", author)).unwrap_or_else(String::new);
                Ok(format!(
                    "<article><h2><a href=\"{}\">{}</a></h2>{}{}</article>",
                    article.get::<_, &str>("path"),
                    article.get::<_, &str>("title"),
                    article.get::<_, &str>("date"),
                    by_author,
                ))
            }
            Pattern::ArticleLatest(no) => {
                let rows = client
                    .query("select path, title, to_char(cdate, 'yyyy-mm-dd') as date, author from articles order by cdate", &[])
                    .await?;
                let article = rows.len().checked_sub(no).and_then(|no| rows.get(no));
                let contents = article.map(|article| {
                    future::ok(article)
                        .and_then(async move |article| {
                            let path = article.get::<_, &str>("path");
                            let path = PublicPath::try_from(path)?;
                            if path.exists() {
                                let text = fs::read_to_string(&path)
                                    .await?;
                                if path.extension() == Some("md".as_ref()) {
                                    let parser = md::Parser::new_ext(&text, md::Options::all());
                                    let mut html = String::new();
                                    md::html::push_html(&mut html, parser);
                                    Ok((article, html))
                                } else {
                                    Ok((article, text))
                                }
                            } else {
                                Err(Error::ResourceNotFound(path.to_string_lossy().to_string()))
                            }
                        })
                });
                if let Some(contents) = contents {
                    contents.and_then(async move |(article, contents)| {
                        let by_author = author(client, article.get::<_, i32>("author")).await?.map(|author| format!(" by {}", author)).unwrap_or_else(String::new);
                        Ok(format!(
                            "<article><h1>{}</h1>{}{}<br/>{}</article>",
                            article.get::<_, &str>("title"),
                            article.get::<_, &str>("date"),
                            by_author,
                            contents,
                        ))
                    }).await
                } else {
                    Ok(String::new())
                }
            }
            Pattern::PreviewTitle(title) => {
                let article = client
                    .query_one("select title, path, to_char(cdate, 'yyyy-mm-dd') as date, author from articles where title = $1", &[&title])
                    .await?;
                let by_author = author(client, article.get::<_, i32>("author")).await?.map(|author| format!(" by {}", author)).unwrap_or_else(String::new);
                Ok(format!(
                    "<article><h2><a href=\"{}\">{}</a></h2>{}{}</article>",
                    article.get::<_, &str>("path"),
                    article.get::<_, &str>("title"),
                    article.get::<_, &str>("date"),
                    by_author,
                ))
            }
            Pattern::ArticleTitle(title) => {
                let args: &[&(dyn psql::types::ToSql + Sync)] = &[&title];
                let article = client
                    .query_one("select title, to_char(cdate, 'yyyy-mm-dd') as date, author from articles where title = $1", args);
                let contents = article
                    .map_err(From::from)
                    .and_then(async move |article| {
                        let path = article.get::<_, &str>("path");
                        let path = PublicPath::try_from(path)?;
                        if path.exists() {
                            let text = fs::read_to_string(&path)
                                .await?;
                            if path.extension() == Some("md".as_ref()) {
                                let parser = md::Parser::new_ext(&text, md::Options::all());
                                let mut html = String::new();
                                md::html::push_html(&mut html, parser);
                                Ok((article, html))
                            } else {
                                Ok((article, text))
                            }
                        } else {
                            Err(Error::ResourceNotFound(path.to_string_lossy().to_string()))
                        }
                    });
                contents.and_then(async move |(article, contents)| {
                    let by_author = author(client, article.get::<_, i32>("author")).await?.map(|author| format!(" by {}", author)).unwrap_or_else(String::new);
                    Ok(format!(
                        "<article><h1>{}</h1>{}{}<br/>{}</article>",
                        article.get::<_, &str>("title"),
                        article.get::<_, &str>("date"),
                        by_author,
                        contents,
                    ))
                }).await
            }
            Pattern::Maybe(_) => {
                Err(Error::AsyncRecursion)
            }
        }
    }

    pub async fn to_string(
        self,
        identity: &Identity,
        client: &psql::Client,
        args: &[String],
    ) -> Result<String> {
        match self {
            Pattern::Maybe(opt) => {
                Ok(opt.to_string_nonrecursive(identity, client, args).await.unwrap_or_else(|_| String::new()))
            }
            other => other.to_string_nonrecursive(identity, client, args).await,
        }
    }

    pub async fn replace_at(
        self,
        identity: &Identity,
        client: &psql::Client,
        input: &mut String,
        start: usize,
        end: usize,
        args: &[String],
    ) -> Result<usize> {
        let text = self.to_string(identity, client, args).await?;
        input.replace_range(start..(end + 3), &text);
        Ok(text.len())
    }
}

async fn replace_at(identity: &Identity, client: &psql::Client, input: &mut String, start: usize, args: &[String]) -> Result<usize> {
    if let Some(len) = &input[start..].find("}}}") {
        let end = start + len;
        let pattern = &input[(start + 3)..end];
        let pattern = pattern.parse().unwrap_or(Pattern::Empty);
        pattern.replace_at(identity, client, input, start, end, args).await
    } else {
        Ok(0)
    }
}

pub async fn search_replace(identity: &Identity, client: &psql::Client, input: &mut String, args: &[String]) -> Result<()> {
    let mut i = 0;
    loop {
        match input[i..].find("{{{") {
            Some(idx) => {
                let len = replace_at(identity, client, input, idx, args).await?;
                i = idx + len;
            }
            None => break Ok(()),
        }
    }
}

pub async fn search_replace_recursive(identity: &Identity, client: &psql::Client, input: &mut String, args: &[String]) -> Result<()> {
    loop {
        match input.find("{{{") {
            Some(idx) => {
                replace_at(identity, client, input, idx, args).await?;
            }
            None => break Ok(()),
        }
    }
}
