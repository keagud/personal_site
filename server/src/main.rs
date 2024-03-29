use axum::{
    routing::{get, post},
    Router, Server,
};

pub mod blog;

pub mod common {
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
        let naive =
            NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

        let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

        dt.format(format_str).to_string()
    }
}

pub mod route {
    use crate::{
        blog::render::read_file_contents,
        common::{self, Post, POSTS_MARKDOWN_PATH},
    };
    use anyhow;
    use anyhow::format_err;
    use axum::{
        extract::{self, Json},
        http::StatusCode,
        response::{Html, IntoResponse},
    };
    use axum_auth::AuthBearer;

    use std::{
        io::Read,
        io::Write,
        path::{Path, PathBuf},
    };

    use bzip2::read::BzDecoder;
    use serde::{Deserialize, Serialize};

    use crate::blog::{db, render};
    use std::fs;

    pub struct SiteError(anyhow::Error, Option<StatusCode>);

    #[derive(Serialize, Deserialize, Default)]
    pub struct PostUpload {
        pub title: String,
        pub timestamp: usize,
        pub slug: String,
        pub file_content_compressed: String,
        pub overwrite: bool,
    }

    impl PostUpload {
        pub fn metadata(&self) -> Post {
            Post {
                title: self.title.to_owned(),
                slug: self.slug.to_owned(),
                timestamp: self.timestamp,
            }
        }

        pub fn save(&self) -> anyhow::Result<()> {
            let upload_bytes = hex::decode(&self.file_content_compressed)?;

            let mut decoder = BzDecoder::new(upload_bytes.as_slice());
            let mut str_buf = String::new();
            decoder.read_to_string(&mut str_buf)?;

            let filename = format!("{}.md", self.slug);

            let save_path = PathBuf::from(POSTS_MARKDOWN_PATH).join(&filename);

            match save_path.try_exists() {
                Err(e) => Err(e.into()),
                Ok(true) if !self.overwrite => Err(format_err!("'{filename}' already exists")),
                _ => Ok(()),
            }?;

            fs::File::create(save_path)?.write_all(str_buf.as_bytes())?;

            let _conn = db::DbConnection::new()?;

            db::DbConnection::new()?.add_post_data(&self.metadata())?;

            Ok(())
        }
    }

    pub struct StaticPage {
        title: String,
        page_path: PathBuf,
    }

    impl StaticPage {
        pub fn new(title: &str, page_path: impl AsRef<Path>) -> Self {
            Self {
                title: title.into(),
                page_path: PathBuf::from(common::STATIC_PAGES_PATH).join(page_path.as_ref()),
            }
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

        let content = render::RenderBuilder::new()
            .html_content(&posts_list)
            .into_base_template("Posts Index")
            .render()?;
        Ok(Html::from(content))
    }

    async fn static_route(page: StaticPage) -> Result<Html<String>, SiteError> {
        let content = render::read_file_contents(page.page_path)
            .and_then(|ref s| {
                render::RenderBuilder::new()
                    .html_content(s)
                    .into_base_template(&page.title)
                    .render()
            })
            .map(Html::from)?;

        Ok(content)
    }

    pub async fn about() -> Result<Html<String>, SiteError> {
        static_route(StaticPage::new("About", "about.html")).await
    }

    pub async fn home() -> Result<Html<String>, SiteError> {
        static_route(StaticPage::new("Home", "homepage.html")).await
    }

    pub async fn get_post(
        extract::Path(slug): extract::Path<String>,
    ) -> Result<Html<String>, SiteError> {
        let post = db::DbConnection::new()?.get(&slug)?;

        let md_content = read_file_contents(post.md_path())?;

        render::RenderBuilder::new()
            .md_content(&md_content)
            .into_base_template(&post.title)
            .render()
            .map(Html::from)
            .map_err(|e| e.into())
    }

    pub async fn add_new_post(
        AuthBearer(token): AuthBearer,
        Json(payload): Json<PostUpload>,
    ) -> Result<StatusCode, SiteError> {
        if !common::validate_token(token) {
            Err(SiteError::from_status(StatusCode::FORBIDDEN))
        } else {
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

    Ok(())
}
