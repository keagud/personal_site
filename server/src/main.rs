use axum::{
    routing::{get, post},
    Router, Server,
};

use axum::http;

use base64::engine::general_purpose;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use md_render::{MdRenderOpts, RenderBuilder};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use sqlx::{sqlite, Executor, Row};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use common::asset;

use anyhow;
use anyhow::format_err;
use handlebars::Handlebars;

#[cfg(debug_assertions)]
const MIGRATIONS_DIR: &'static str = "./migrations";

#[cfg(not(debug_assertions))]
const MIGRATIONS_DIR: &str = "";

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, FromRow)]
pub struct Post {
    pub title: String,
    pub timestamp: i64,
    pub slug: String,
    pub content: String,
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
pub const POSTS_JSON_PATH: &str = asset!("/posts.json");
pub const POSTS_FILES_PATH: &str = asset!("/posts");
pub const TEMPLATES_PATH: &str = asset!("/templates");

pub const HOMEPAGE_PATH: &str = asset!("/static/homepage.html");

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

pub struct Connection {}

pub struct PostMetadata {
    pub title: String,
    pub slug: String,
    pub timestamp: i64,
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
    .bind(&post.timestamp)
    .bind(&post.slug)
    .bind(&post.content)
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

pub fn read_file_contents(file_path: impl AsRef<Path>) -> anyhow::Result<String> {
    let file_path = PathBuf::from(file_path.as_ref());

    let mut file_handle = match file_path.metadata() {
        Err(e) => Err(anyhow::Error::from(e)),
        Ok(m) if m.is_dir() => Err(format_err!("bad")),
        Ok(_) => match File::open(file_path) {
            Ok(f) => Ok(f),
            Err(e) => Err(anyhow::Error::from(e)),
        },
    }?;

    let mut buf: Vec<u8> = Vec::new();
    file_handle.read_to_end(&mut buf)?;

    Ok(String::from_utf8(buf)?)
}

fn get_template_path(template_name: &str) -> anyhow::Result<PathBuf> {
    let p = PathBuf::from(TEMPLATES_PATH);
    let template_filename = format!("{}.html", template_name);
    let template_path = p.join(template_filename).canonicalize()?;

    if template_path.try_exists()? {
        Ok(template_path)
    } else {
        Err(format_err!(
            "Template file not found or not accessable at {:?}",
            template_path
        ))
    }
}

pub fn post_index_display(posts: &Vec<Post>) -> anyhow::Result<String> {
    let mut hb = Handlebars::new();
    let template_path = get_template_path("posts_list")?;

    hb.register_template_file("posts_list", template_path)?;

    let mut template_values = serde_json::Map::new();
    let list_items_json = handlebars::to_json(posts);
    template_values.insert(String::from("posts"), list_items_json);

    let rendered_content = hb.render("posts_list", &template_values)?;

    Ok(dbg!(rendered_content))
}

pub fn render_post(post: &Post) -> anyhow::Result<PathBuf> {
    let post_path = PathBuf::from(POSTS_FILES_PATH)
        .join(&post.slug)
        .with_extension("html");

    if !post_path.try_exists().is_ok_and(|t| !t) {
        return Err(format_err!("File '{}.html' already exists", post.title));
    }

    let rendered_content = RenderBuilder::new()
        .md_content(&post.content)
        .sidenotes()
        .into_base_template(&post.title)
        .render()?;

    fs::write(post_path.as_path(), &rendered_content)?;

    Ok(post_path)
}

pub struct Routes {
    db_pool: SqlitePool,
}

impl Routes {
    pub async fn new() -> Self {
        let db_pool = init_db().await.unwrap();
        Self { db_pool }
    }
}

impl Routes {
    fn serve_static(filename: impl AsRef<Path>) {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    //TODO figure out how to use middleware to avoid
    //needing to specify path versions with and without slashes
    /*
    let app = Router::new()
        .route("/", get(route::home))
        .route("/blog", get(route::posts_list))
        .route("/blog/", get(route::posts_list))
        .route("/about", get(route::about))
        .route("/about/", get(route::about))
        .route("/admin/add", post(route::add_new_post))
        //.route("/admin/posts", get(route::admin_posts_list))
        .route("/blog/:slug", get(route::get_post))
        .route("/blog/:slug/", get(route::get_post));

    let port = "8000";
    let host = "0.0.0.0";

    let addr = format!("{host}:{port}");
    tracing::debug!("Listening on {}", &addr);
    let res = Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await;

    if let Some(e) = res.err() {
        eprintln!("ERROR: {:?}", e);
    }
    */

    Ok(())
}
