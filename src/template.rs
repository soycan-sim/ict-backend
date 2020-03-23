use pulldown_cmark as md;
use tokio::fs;
use tokio_postgres as psql;

use super::Result;

#[derive(Debug, Clone)]
enum Pattern {
    Path(String),
    Positional(usize),
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
                let text = fs::read_to_string(format!("public/{}", path)).await?;
                if path.ends_with(".md") {
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
                let text = fs::read_to_string(format!("public/{}", path)).await?;
                if path.ends_with(".md") {
                    let parser = md::Parser::new_ext(&text, md::Options::all());
                    let mut html = String::new();
                    md::html::push_html(&mut html, parser);
                    html
                } else {
                    text
                }
            }
            Pattern::PreviewLatest(no) => {
                let rows = client
                    .query("select title, date, author from articles order by date", &[])
                    .await?;
                let article = if no <= rows.len() {
                    &rows[no - 1]
                } else {
                    return Ok(()); // quietly ignore errors
                };
                format!(
                    "<article><h2>{}</h2>{} ~{}</article>",
                    article.get::<_, &str>("title"),
                    article.get::<_, &str>("date"),
                    article.get::<_, &str>("author"),
                )
            }
            Pattern::ArticleLatest(no) => {
                let rows = client
                    .query("select path, title, date, author from articles order by date", &[])
                    .await?;
                let article = if no <= rows.len() {
                    &rows[no - 1]
                } else {
                    return Ok(()); // quietly ignore errors
                };
                let contents = {
                    let path = article.get::<_, &str>("path");
                    let text = fs::read_to_string(format!("public/{}", path)).await?;
                    if path.ends_with(".md") {
                        let parser = md::Parser::new_ext(&text, md::Options::all());
                        let mut html = String::new();
                        md::html::push_html(&mut html, parser);
                        html
                    } else {
                        text
                    }
                };
                format!(
                    "<article><h2>{}</h2>{} ~{}<br/>{}</article>",
                    article.get::<_, &str>("title"),
                    article.get::<_, &str>("date"),
                    article.get::<_, &str>("author"),
                    contents,
                )
            }
            Pattern::PreviewTitle(title) => {
                let article = client
                    .query_one("select title, date, author from articles where title = $1", &[&title])
                    .await?;
                format!(
                    "<article><h2>{}</h2>{} ~{}</article>",
                    article.get::<_, &str>("title"),
                    article.get::<_, &str>("date"),
                    article.get::<_, &str>("author"),
                )
            }
            Pattern::ArticleTitle(title) => {
                let article = client
                    .query_one("select title, date, author from articles where title = $1", &[&title])
                    .await?;
                let contents = {
                    let path = article.get::<_, &str>("path");
                    let text = fs::read_to_string(format!("public/{}", path)).await?;
                    if path.ends_with(".md") {
                        let parser = md::Parser::new_ext(&text, md::Options::all());
                        let mut html = String::new();
                        md::html::push_html(&mut html, parser);
                        html
                    } else {
                        text
                    }
                };
                format!(
                    "<article><h2>{}</h2>{} ~{}<br/>{}</article>",
                    article.get::<_, &str>("title"),
                    article.get::<_, &str>("date"),
                    article.get::<_, &str>("author"),
                    contents,
                )
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
