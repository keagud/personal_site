use axum::{routing::get, Router, Server};

pub mod blog;

pub mod common {
    use serde::{Deserialize, Serialize};
    use sha3::{Digest, Sha3_512};
    use std::path::PathBuf;

    use chrono::{DateTime, NaiveDateTime, Utc};
    use dotenv_codegen::dotenv;

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

    pub fn validate_token(token: impl AsRef<[u8]>) -> bool {
        const SECRET: &str = dotenv!("SECRET");

        /*
        let cleaned_bytes = match String::from_utf8(SECRET.into()) {
            Ok(ref s) => s.trim().as_bytes().to_owned(),
            _ => return false,
        };
        */

        let mut hasher = Sha3_512::new();
        hasher.update(SECRET);
        let result = hasher.finalize().into_iter().collect::<Vec<u8>>();

        result.as_slice() == token.as_ref()
    }

    pub fn timestamp_date_format(timestamp: usize, format_str: &str) -> String {
        let naive =
            NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

        let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

        dt.format(format_str).to_string()
    }


    #[cfg(test)]
    pub mod test{

        use crate::common::*;
        use dotenv_codegen::dotenv;


        #[test]
        fn test_validator(){

            const KEY: &str = dotenv!("KEY");

            assert!(validate_token(KEY));




        }


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
            .map(Html::from)?;

        Ok(content)
    }

    pub async fn post(
        extract::Path(slug): extract::Path<String>,
    ) -> Result<Html<String>, RoutingError> {
        let post = db::DbConnection::new()?.get(&slug)?;
        let content = render::render_md(&post.title, post.md_path())?;

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
