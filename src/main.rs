use axum::{
    headers::authorization::Bearer,
    routing::{get, post},
    Json, Router, Server, TypedHeader,
};

pub mod blog;

pub mod common {
    use hex;
    use html5ever::tokenizer::Token::DoctypeToken;
    use serde::{Deserialize, Serialize};
    use sha3::{Digest, Sha3_512};
    use std::option_env;
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

        let secret_bytes =
            hex::decode(SECRET).expect("Hardcoded secret hex should represent valid bytes");

        if let Ok(token_bytes) = hex::decode(&token) {
            let mut hasher = Sha3_512::new();
            hasher.update(secret_bytes);
            let result = hasher.finalize();

            result.as_slice() == token_bytes
        } else {
            false
        }
    }

    pub fn decode_base64(encoded: impl AsRef<[u8]>) -> anyhow::Result<String> {
        todo!()
    }

    pub fn timestamp_date_format(timestamp: usize, format_str: &str) -> String {
        let naive =
            NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

        let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

        dt.format(format_str).to_string()
    }

    #[cfg(test)]
    pub mod test {

        use crate::common::*;
        use dotenv_codegen::dotenv;

        #[test]
        fn test_validator() {
            const KEY: &str = dotenv!("KEY");

            assert!(validate_token(KEY));
        }
    }
}

pub mod route {
    use crate::common::{self, Post};
    use anyhow;
    use anyhow::format_err;
    use axum::{
        body::{Body, HttpBody},
        extract::{self, Json, Multipart},
        headers::authorization::Bearer,
        http::{Request, StatusCode},
        response::{Html, IntoResponse},
        TypedHeader,
    };

    use serde::{Deserialize, Serialize};

    use crate::blog::{db, render};
    pub struct SiteError(anyhow::Error, Option<StatusCode>);

    #[derive(Serialize, Deserialize, Default)]
    pub struct PostUpload {
        pub title: String,
        pub timestamp: usize,
        pub slug: String,
        pub file_content_base64: String,
    }

    impl PostUpload {
        pub fn save(&self) -> anyhow::Result<Post> {
            // decode file content from base 64, save it
            // to the correct path to a file named slug.md.
            // Return a Post on success.
            todo!()
        }
    }

    impl SiteError {
        pub fn from_status(s: StatusCode) -> Self {
            Self(format_err!("{:?}", s), Some(s))
        }
    }

    impl<E> From<E> for SiteError
    where
        E: Into<anyhow::Error>,
    {
        fn from(err: E) -> Self {
            Self(err.into(), None)
        }
    }
    impl IntoResponse for SiteError {
        fn into_response(self) -> axum::response::Response {
            if cfg!(debug_assertions) {
                tracing::debug!("{}", self.0.backtrace());
            }

            (
                self.1.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                format!("Something went wrong: {:?}", self.0),
            )
                .into_response()
        }
    }

    pub async fn posts_list() -> Result<Html<String>, SiteError> {
        let posts = db::DbConnection::new()?.all_posts()?;
        let posts_list = render::post_index_display(&posts)?;
        let content = render::render_html_str("Posts Index", &posts_list)?;
        Ok(Html::from(content))
    }

    pub async fn home() -> Result<Html<String>, SiteError> {
        let content = render::read_file_contents(common::HOMEPAGE_PATH)
            .and_then(|ref s| render::render_html_str("Home", s))
            .map(Html::from)?;

        Ok(content)
    }

    pub async fn get_post(
        extract::Path(slug): extract::Path<String>,
    ) -> Result<Html<String>, SiteError> {
        let post = db::DbConnection::new()?.get(&slug)?;
        let content = render::render_md(&post.title, post.md_path())?;

        Ok(Html::from(content))
    }

    pub async fn add_new_post(
        TypedHeader(bearer_auth): TypedHeader<Bearer>,
        Json(payload): Json<PostUpload>,
    ) -> Result<StatusCode, SiteError> {
        if !common::validate_token(bearer_auth.token()) {
            Err(SiteError::from_status(StatusCode::FORBIDDEN))
        } else {
            // TODO save to database here
            payload.save().map(|_| StatusCode::OK).map_err(|e| e.into())
        }
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

    Ok(())
}
