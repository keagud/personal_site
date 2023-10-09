use warp;
use warp::Filter;

use sqlx;
use sqlx::sqlite;
use sqlx::sqlite::SqlitePool;

use anyhow;
use anyhow::format_err;

use base64::engine::general_purpose;

use base64::Engine;

use serde::{Deserialize, Serialize};

use std::path::PathBuf;

use chrono::{DateTime, NaiveDateTime, Utc};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Post {
    pub title: String,
    pub timestamp: usize,
    pub slug: String,
    pub text_content: String,
    pub text_rendered: String,
}

impl Post {
    pub fn date_str(&self) -> String {
        timestamp_date_format(self.timestamp, "%F")
    }

    pub fn md_path(&self) -> PathBuf {
        PathBuf::from(POSTS_MARKDOWN_PATH).join(format!("{}.md", self.slug))
    }

    pub fn html_path(&self) -> PathBuf {
        PathBuf::from(POSTS_FILES_PATH).join(format!("{}.html", self.slug))
    }
}

//relative to crate root
//TODO make this work relative to file rather than cwd,
//so it can be invoked from anywhere
pub const POSTS_DB_PATH: &str = "../assets/posts.db";
pub const POSTS_JSON_PATH: &str = "../assets/posts.json";
pub const POSTS_FILES_PATH: &str = "../assets/posts/html";
pub const POSTS_MARKDOWN_PATH: &str = "../assets/posts/md";
pub const TEMPLATES_PATH: &str = "../assets/templates";

pub const STATIC_PAGES_PATH: &str = "../assets/static";
pub const HOMEPAGE_PATH: &str = "../assets/static/homepage.html";

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

pub fn timestamp_date_format(timestamp: usize, format_str: &str) -> String {
    let naive = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

    let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

    dt.format(format_str).to_string()
}

pub struct Connection {}

async fn init_db() -> anyhow::Result<()> {
    let pool = sqlite::SqlitePoolOptions::new().max_connections(5);

    Ok(())
}

pub async fn add_post(pool: &SqlitePool, post: &Post) -> anyhow::Result<()> {
    todo!();
}

pub async fn fetch_post(pool: &SqlitePool, slug: &str) -> anyhow::Result<Post> {
    todo!();
}

pub mod routes {
    use warp;
}

#[tokio::main]
async fn main() {
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {name}!"));

    warp::serve(hello).run(([127, 0, 0, 1], 8000)).await;
}
