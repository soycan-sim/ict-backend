use std::convert::TryFrom;

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
    Login,
    Me(String),
    Path(String),
    Positional(usize),
    ArticlePositional(usize),
    PreviewLatest(usize),
    ArticleLatest(usize),
    PreviewTitle(String),
    ArticleTitle(String),
}

impl Pattern {
    pub async fn replace_at(
        self,
        identity: &Identity,
        client: &psql::Client,
        input: &mut String,
        start: usize,
        end: usize,
        args: &[String],
    ) -> Result<usize> {
        let text = match self {
            Pattern::Login => {
                match identity.identity() {
                    Some(identity) => {
                        format!("<span class=\"float-right\"><a href=\"/account/me.html\">Logged in as: {}</a></span> \
                                 <span class=\"float-right\"><a href=\"/auth/logout.html\">Logout</a></span>", identity)
                    }
                    None => {
                        "<span class=\"float-right\"><a href=\"/create.html\">Register</a></span> \
                         <span class=\"float-right\"><a href=\"/login.html\">Login</a></span>".to_string()
                    }
                }
            }
            Pattern::Me(field) => {
                if field == "pwhash" {
                    "No passwords for you!".to_string()
                } else {
                    match identity.identity() {
                        Some(me) => {
                            match client.query_opt("select * from users where username = $1", &[&me]).await? {
                                Some(row) => row.get::<&str, &str>(&field).to_string(),
                                None => "".to_string(),
                            }
                        }
                        None => "".to_string(),
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
                    html
                } else {
                    text
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
                    html
                } else {
                    text
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
                contents.map_ok(|(article, contents)| {
                    format!(
                        "<article><h2>{}</h2>{} ~{}<br/>{}</article>",
                        article.get::<_, &str>("title"),
                        article.get::<_, &str>("date"),
                        article.get::<_, &str>("author"),
                        contents,
                    )
                }).await?
            }
            Pattern::PreviewLatest(no) => {
                let rows = client
                    .query("select title, path, to_char(cdate, 'yyyy-mm-dd') as date, author from articles order by cdate", &[])
                    .await?;
                let article = rows.get(no - 1);
                article.map(|article| {
                    format!(
                        "<article><h2><a href=\"{}\">{}</a></h2>{} ~{}</article>",
                        article.get::<_, &str>("path"),
                        article.get::<_, &str>("title"),
                        article.get::<_, &str>("date"),
                        article.get::<_, &str>("author"),
                    )
                }).unwrap_or_else(String::new)
            }
            Pattern::ArticleLatest(no) => {
                let rows = client
                    .query("select path, title, to_char(cdate, 'yyyy-mm-dd') as date, author from articles order by cdate", &[])
                    .await?;
                let article = rows.get(no - 1);
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
                    contents.map_ok(|(article, contents)| {
                        format!(
                            "<article><h2>{}</h2>{} ~{}<br/>{}</article>",
                            article.get::<_, &str>("title"),
                            article.get::<_, &str>("date"),
                            article.get::<_, &str>("author"),
                            contents,
                        )
                    }).await?
                } else {
                    String::new()
                }
            }
            Pattern::PreviewTitle(title) => {
                let article = client
                    .query_opt("select title, path, to_char(cdate, 'yyyy-mm-dd') as date, author from articles where title = $1", &[&title])
                    .await?;
                article.map(|article| {
                    format!(
                        "<article><h2><a href=\"{}\">{}</a></h2>{} ~{}</article>",
                        article.get::<_, &str>("path"),
                        article.get::<_, &str>("title"),
                        article.get::<_, &str>("date"),
                        article.get::<_, &str>("author"),
                    )
                }).unwrap_or_else(String::new)
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
                contents.map_ok(|(article, contents)| {
                    format!(
                        "<article><h2>{}</h2>{} ~{}<br/>{}</article>",
                        article.get::<_, &str>("title"),
                        article.get::<_, &str>("date"),
                        article.get::<_, &str>("author"),
                        contents,
                    )
                }).await?
            }
        };
        input.replace_range(start..(end + 3), &text);
        Ok(text.len())
    }
}

async fn replace_at(identity: &Identity, client: &psql::Client, input: &mut String, start: usize, args: &[String]) -> Result<usize> {
    if let Some(len) = &input[start..].find("}}}") {
        let end = start + len;
        let pattern = &input[(start + 3)..end];
        let pattern = if pattern == "login" {
            Pattern::Login
        } else if pattern.starts_with("me.") {
            Pattern::Me(pattern[3..].to_string())
        } else if pattern.starts_with('/') {
            Pattern::Path(pattern[1..].to_string())
        } else if pattern.starts_with('%') {
            Pattern::Positional(pattern[1..].parse()?)
        } else if pattern.starts_with("article%") {
            Pattern::ArticlePositional(pattern["article%".len()..].parse()?)
        } else if pattern.starts_with("preview~") {
            Pattern::PreviewLatest(pattern["preview~".len()..].parse()?)
        } else if pattern.starts_with("article~") {
            Pattern::ArticleLatest(pattern["article~".len()..].parse()?)
        } else if pattern.starts_with("preview ") {
            Pattern::PreviewTitle(pattern["preview ".len()..].to_string())
        } else if pattern.starts_with("article ") {
            Pattern::ArticleTitle(pattern["article ".len()..].to_string())
        } else {
            return Ok(0);
        };
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
