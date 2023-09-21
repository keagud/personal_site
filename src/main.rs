use axum::{routing::get, Router, Server};
use tracing::{error, instrument};
use tracing_error::ErrorLayer;

pub mod blog;

use blog::{db, render};

pub mod common {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    use chrono::{DateTime, NaiveDateTime, Utc};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Post {
        pub title: String,
        pub timestamp: usize,
        pub slug: String,
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
    pub static POSTS_DB_PATH: &str = "./assets/posts.db";
    pub static POSTS_JSON_PATH: &str = "./assets/posts.json";
    pub static POSTS_FILES_PATH: &str = "./assets/posts/html";
    pub static POSTS_MARKDOWN_PATH: &str = "./assets/posts/md";
    pub static TEMPLATES_PATH: &str = "./assets/templates";
    pub static STATIC_PAGES_PATH: &str = "./assets/static";
    pub static HOMEPAGE_PATH: &str = "./assets/static/homepage.html";

    pub fn timestamp_date_format(timestamp: usize, format_str: &str) -> String {
        let naive =
            NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

        let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

        dt.format(format_str).to_string()
    }
}

pub mod route {
    use crate::common;
    use anyhow;
    use axum::{
        extract,
        http::StatusCode,
        response::{Html, IntoResponse},
    };

    use crate::blog::{db, render};
    pub struct RoutingError(anyhow::Error);

    impl<E> From<E> for RoutingError
    where
        E: Into<anyhow::Error>,
    {
        fn from(err: E) -> Self {
            Self(err.into())
        }
    }
    impl IntoResponse for RoutingError {
        fn into_response(self) -> axum::response::Response {
            if cfg!(debug_assertions) {
                tracing::debug!("{}", self.0.backtrace());
            }

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {:?}", self.0),
            )
                .into_response()
        }
    }

    pub async fn posts_list() -> Result<Html<String>, RoutingError> {
        let posts = db::DbConnection::new()?.all_posts()?;
        let posts_list = render::post_index_display(&posts)?;
        let content = render::render_html_str("Posts Index", &posts_list)?;
        Ok(Html::from(content))
    }

    pub async fn home() -> Result<Html<String>, RoutingError> {
        let content = render::read_file_contents(common::HOMEPAGE_PATH)
            .and_then(|ref s| render::render_html_str("Home", s))
            .map(|s| Html::from(s))?;

        Ok(content)
    }

    pub async fn post(
        extract::Path(slug): extract::Path<String>,
    ) -> Result<Html<String>, RoutingError> {
        let post = db::DbConnection::new()?.get(&slug)?;
        let content = render::render_md(&post.title, &post.md_path())?;

        Ok(Html::from(content))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    //TODO figure out how to use middleware to avoid
    //needing to specify path versions with and without slashes
    let app = Router::new()
        .route("/", get(route::home))
        .route("/blog", get(route::posts_list))
        .route("/blog/", get(route::posts_list))
        .route("/blog/:slug", get(route::post))
        .route("/blog/:slug/", get(route::post));

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

    Ok(())
}
