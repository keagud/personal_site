use base64::engine::general_purpose;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use md_render::RenderBuilder;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use sqlx::Row;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use common::asset;

use anyhow::format_err;
use handlebars::Handlebars;

#[cfg(debug_assertions)]
const MIGRATIONS_DIR: &str = "./migrations";

#[cfg(not(debug_assertions))]
const MIGRATIONS_DIR: &str = "";

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
pub const POSTS_DB_PATH: &str = asset!("/posts.db");
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

fn main() {
    println!("Hello, world!");
}

pub mod db {

    use crate::Post;
    use anyhow;
    use sqlx::sqlite::SqlitePool;
    use sqlx::{sqlite, Executor, Row};

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
}
