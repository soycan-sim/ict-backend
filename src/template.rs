use futures::future;
use futures::TryFutureExt;
use pulldown_cmark as md;
use tokio::fs;
use tokio_postgres as psql;

use crate::path::PublicPath;
use super::Result;

#[derive(Debug, Clone)]
enum Pattern {
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
        client: &psql::Client,
        input: &mut String,
        start: usize,
        end: usize,
        args: &[&str],
    ) -> Result<()> {
        let text = match self {
            Pattern::Path(path) => {
                let path = PublicPath::from(path);
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
                let path = args[pos - 1];
                let path = PublicPath::from(path);
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
                let path = args[pos - 1];
                let article = client
                    .query_opt("select title, to_char(cdate, 'yyyy-mm-dd') as date, author from articles where path = $1", &[&path])
                    .await?;
                let contents = article
                    .map(future::ok)
                    .unwrap_or_else(|| future::err(()))
                    .and_then(async move |article| {
                        let path = PublicPath::from(path);
                        if path.exists() {
                            let text = fs::read_to_string(&path)
                                .await
                                .map_err(|_| ())?;
                            if path.extension() == Some("md".as_ref()) {
                                let parser = md::Parser::new_ext(&text, md::Options::all());
                                let mut html = String::new();
                                md::html::push_html(&mut html, parser);
                                Ok((article, html))
                            } else {
                                Ok((article, text))
                            }
                        } else {
                            Err(()) // TODO: 404 error
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
                }).await
                    .unwrap_or_else(|_| String::new())
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
                let contents = article
                    .map(future::ok)
                    .unwrap_or_else(|| future::err(()))
                    .and_then(async move |article| {
                        let path = article.get::<_, &str>("path");
                        let path = PublicPath::from(path);
                        if path.exists() {
                            let text = fs::read_to_string(&path)
                                .await
                                .map_err(|_| ())?;
                            if path.extension() == Some("md".as_ref()) {
                                let parser = md::Parser::new_ext(&text, md::Options::all());
                                let mut html = String::new();
                                md::html::push_html(&mut html, parser);
                                Ok((article, html))
                            } else {
                                Ok((article, text))
                            }
                        } else {
                            Err(())
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
                }).await
                    .unwrap_or_else(|_| String::new())
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
                let article = client
                    .query_opt("select title, to_char(cdate, 'yyyy-mm-dd') as date, author from articles where title = $1", &[&title])
                    .await?;
                let contents = article
                    .map(future::ok)
                    .unwrap_or_else(|| future::err(()))
                    .and_then(async move |article| {
                    let path = article.get::<_, &str>("path");
                    let path = PublicPath::from(path);
                    if path.exists() {
                        let text = fs::read_to_string(&path)
                            .await
                            .map_err(|_| ())?;
                        if path.extension() == Some("md".as_ref()) {
                            let parser = md::Parser::new_ext(&text, md::Options::all());
                            let mut html = String::new();
                            md::html::push_html(&mut html, parser);
                            Ok((article, html))
                        } else {
                            Ok((article, text))
                        }
                    } else {
                        Err(())
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
                }).await
                    .unwrap_or_else(|_| String::new())
            }
        };
        input.replace_range(start..(end + 3), &text);
        Ok(())
    }
}

async fn replace_at(client: &psql::Client, input: &mut String, start: usize, args: &[&str]) -> Result<()> {
    if let Some(len) = &input[start..].find("}}}") {
        let end = start + len;
        let pattern = &input[(start + 3)..end];
        let pattern = if pattern.starts_with('/') {
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
            return Ok(());
        };
        pattern.replace_at(client, input, start, end, args).await
    } else {
        Ok(())
    }
}

pub async fn search_replace(client: &psql::Client, input: &mut String, args: &[&str]) -> Result<()> {
    loop {
        match input.find("{{{") {
            Some(idx) => {
                replace_at(client, input, idx, args).await?;
            }
            None => break Ok(()),
        }
    }
}
