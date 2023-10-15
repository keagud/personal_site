use base64::engine::general_purpose;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use md_render::RenderBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::sqlite;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use sqlx::Row;
use url;

use markdown;

use std::sync::Arc;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::format_err;
use handlebars::Handlebars;

#[cfg(debug_assertions)]
const MIGRATIONS_DIR: &str = "./migrations";

#[cfg(not(debug_assertions))]
const MIGRATIONS_DIR: &str = "";

macro_rules! crate_root {
    ($x:literal) => {
        concat!(std::env!("CARGO_MANIFEST_DIR"), $x)
    };
}
macro_rules! static_file {
    ($x:literal) => {
        concat!(std::env!("CARGO_MANIFEST_DIR"), "/assets", $x)
    };
}

//Eventually this will point to where the static files are hosted
//for now it just formats a filepath into a uri
macro_rules! static_url {
    ($x:literal) => {
        path_to_url(static_file!($x))
            .map_err(|e| {
                println!("Failed to find file {} {:?}", $x, e);
                e
            })
            .unwrap_or("".into())
    };
}

pub fn path_to_url(filepath: impl AsRef<Path>) -> anyhow::Result<String> {
    let p: PathBuf = filepath.as_ref().canonicalize()?;
    let path_str = p.to_str().ok_or(format_err!("Invalid path: {:?}", p))?;

    Ok(format!(r"file://{path_str}"))
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, FromRow)]
pub struct Post {
    pub title: String,
    pub timestamp: i64,
    pub slug: String,
    pub text_content: String,
    pub text_rendered: Option<String>,
}

impl Post {
    pub fn date_str(&self) -> String {
        timestamp_date_format(self.timestamp, "%F")
    }

    pub fn html_path(&self) -> PathBuf {
        PathBuf::from(POSTS_FILES_PATH).join(format!("{}.html", self.slug))
    }
}

//relative to crate root
//TODO make this work relative to file rather than cwd,
//so it can be invoked from anywhere
pub const POSTS_DB_PATH: &str = crate_root!("/posts.db");
pub const POSTS_JSON_PATH: &str = static_file!("/posts.json");
pub const POSTS_FILES_PATH: &str = static_file!("/posts");
pub const TEMPLATES_PATH: &str = static_file!("/templates");

pub const HOMEPAGE_PATH: &str = static_file!("/static/homepage.html");

pub fn validate_token(token: impl AsRef<[u8]>) -> bool {
    if cfg!(debug_assertions) {
        true
    } else {
        let env_token = std::option_env!("SITE_ADMIN_KEY")
            .expect("Admin key should be present in release builds");

        env_token.as_bytes() == token.as_ref()
    }
}

pub fn decode_base64(encoded: &impl AsRef<[u8]>) -> anyhow::Result<String> {
    let decoded_bytes = general_purpose::STANDARD_NO_PAD.decode(encoded)?;
    let decoded_string = String::from_utf8(decoded_bytes)?;
    Ok(decoded_string)
}

pub fn timestamp_date_format(timestamp: i64, format_str: &str) -> String {
    let naive = NaiveDateTime::from_timestamp_opt(timestamp, 0).expect("Timestamp is valid");

    let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

    dt.format(format_str).to_string()
}

pub struct PostMetadata {
    pub title: String,
    pub slug: String,
    pub timestamp: i64,
}

impl PostMetadata {
    pub fn date_str(&self) -> String {
        timestamp_date_format(self.timestamp, "%F")
    }
}

pub fn markdown_to_html(s: &str) -> anyhow::Result<String> {
    let r = markdown::to_html_with_options(s, &markdown::Options::gfm())
        .map_err(|e| format_err!("{e}"))?;

    Ok(dbg!(r))
}

pub fn process_sidenotes(document_input: &str) -> String {
    let mut document = String::from(document_input);
    let pattern_str = r"\(:sidenote(?<text>.*?):sidenote\)".to_string();
    let mut counter: usize = 1;
    let re = regex::RegexBuilder::new(&pattern_str)
        .dot_matches_new_line(true)
        .build()
        .unwrap();

    while re.is_match(&document) {
        let mn_id = format!("mn-{counter}");
        counter += 1;

        let replacement = format!(
            r#"<label for="{mn_id}" class="margin-toggle"> &#8853;</label> 
            <input type="checkbox" id="{mn_id}" class="margin-toggle"/>
            <span class="marginnote">
            $text
            </span> "#
        );

        document = re.replace(&document, replacement).to_string();
    }

    document
}

pub fn format_posts_list(posts: &Vec<PostMetadata>, hb: &Handlebars) -> anyhow::Result<String> {
    let posts_values: Vec<serde_json::Value> = posts
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "title" : &p.title,
                "date" : &p.date_str(),
                "slug" : &p.slug
            })
        })
        .collect();

    let template_params = serde_json::json!({"posts" : posts_values});

    let _ = hb
        .get_template("posts_list")
        .expect("posts_list template should be registered");

    hb.render("posts_list", &template_params)
        .map_err(|e| e.into())
}

pub fn html_into_base_template(
    title: &str,
    content: &str,
    hb: &Handlebars,
) -> anyhow::Result<String> {
    let template_params = json!(
        {
            "title": title,
            "content" : content,
            "favicon_url" : static_url!("/favicon.ico"),
            "quotes_json_url": static_url!("/quotes.json"),
            "css_url" : static_url!("/css/style.css")
    }
    );

    let _ = hb
        .get_template("base")
        .expect("Base template should be registered");

    hb.render("base", &template_params).map_err(|e| e.into())
}

pub async fn init_db() -> Result<sqlite::SqlitePool, sqlx::Error> {
    let opts = sqlite::SqliteConnectOptions::new()
        .filename("posts.db")
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(opts).await?;
    Ok(pool)
}

pub async fn add_post(pool: &SqlitePool, post: &Post) -> anyhow::Result<()> {
    let _ = sqlx::query(
        "INSERT INTO post (title, timestamp, slug, text_content) VALUES (?1, ?2, ?3, ?4);",
    )
    .bind(&post.title)
    .bind(post.timestamp)
    .bind(&post.slug)
    .bind(&post.text_content)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn fetch_post(pool: &SqlitePool, slug: &str) -> anyhow::Result<Option<Post>> {
    let q = sqlx::query_as::<_, Post>("SELECT * from post WHERE slug = ?1").bind(slug);
    q.fetch_optional(pool).await.map_err(|e| e.into())
}

pub async fn fetch_all_post_metadata(pool: &SqlitePool) -> anyhow::Result<Vec<PostMetadata>> {
    let posts: Vec<PostMetadata> = sqlx::query("SELECT * FROM post ORDER BY timestamp")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| PostMetadata {
            title: row.get("title"),
            timestamp: row.get("timestamp"),
            slug: row.get("slug"),
        })
        .collect();

    Ok(posts)
}

//#[async_std::main]
async fn tmain() -> tide::Result<()> {
    let conn = Arc::new(
        init_db()
            .await
            .expect("Database initialization shouldn't fail"),
    );

    let mut app = tide::with_state(conn);

    let _ = app
        .at("/")
        .serve_file(static_file!("/static/homepage.html"));
    app.at("/about")
        .serve_file(static_file!("/static/about.html"));
    app.listen("0.0.0:8000").await?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let test_md = r#"
# In Catalinam

## Quo usque tandem?
Llorum ipsum dolor ~~sit~~ *amat*

    "#;

    let test_html = markdown_to_html(test_md).unwrap();

    println!("{}", &test_html);

    let mut hb = Handlebars::new();
    hb.register_template_file("base", static_file!("/templates/base.html"))
        .unwrap();

    let rendered_template = html_into_base_template("The Title", &test_html, &hb).unwrap();

    let output_file = concat!(std::env!("CARGO_MANIFEST_DIR"), "/_test.html");

    println!("{}", &rendered_template);

    let mut f = File::create(output_file).unwrap();

    f.write_all(&rendered_template.as_bytes()).unwrap();

    Ok(())
}
