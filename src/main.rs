use axum::{routing::get, Router, Server};

pub mod blog;

pub mod common {

    use chrono::{DateTime, NaiveDateTime, Utc};

    //relative to crate root
    pub static POSTS_DB_PATH: &str = "./assets/posts.db";
    pub static POSTS_FILES_PATH: &str = "./assets/posts/html";
    pub static POSTS_MARKDOWN_PATH: &str = "./assets/posts/md";
    pub static TEMPLATES_PATH: &str = "./assets/templates";

    pub fn timestamp_date_format(timestamp: usize, format_str: &str) -> String {
        let naive =
            NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

        let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

        dt.format(format_str).to_string()
    }
}

#[allow(dead_code)]
fn test_main() -> anyhow::Result<()> {
    let conn = blog::init_table_connection()?;

    let first_post = blog::Post {
        title: String::from("Shave your head!"),
        timestamp: 1687935600,
        slug: String::from("bald"),
    };

    let _ = blog::add_post_metadata_to_db(&conn, &first_post);

    let ip = blog::IndexPage::from_db(&conn)?;

    let content = blog::render_into_base(&ip)?;
    println!("{ip:?}");
    println!("{content}");

    Ok(())
}

pub mod route {

    use crate::common;
    use axum::{
        extract::Path,
        http::StatusCode,
        response::{Html, IntoResponse},
    };
    use std::{fs::File, io::Read};

    use crate::blog;
    use anyhow;
    use anyhow::format_err;
    use std::path::PathBuf;

    pub struct RouteError(anyhow::Error);

    impl IntoResponse for RouteError {
        fn into_response(self) -> axum::response::Response {
            (StatusCode::NOT_FOUND, format!("{:?}", self.0)).into_response()
        }
    }

    impl<E> From<E> for RouteError
    where
        E: Into<anyhow::Error>,
    {
        fn from(err: E) -> Self {
            Self(err.into())
        }
    }

    type PageResult = Result<Html<String>, RouteError>;

    async fn get_static_file_for_slug(slug: &str) -> anyhow::Result<Html<String>> {
        let post_filename = format!("{slug}.html");
        let post_path = PathBuf::from(common::POSTS_FILES_PATH).join(post_filename);

        let mut post_handle = match post_path.metadata() {
            Err(e) => Err(anyhow::Error::from(e)),
            Ok(m) if m.is_dir() => Err(format_err!("bad")),
            Ok(_) => match File::open(post_path) {
                Ok(f) => Ok(f),
                Err(e) => Err(anyhow::Error::from(e)),
            },
        }?;

        let mut buf: Vec<u8> = Vec::new();
        post_handle.read_to_end(&mut buf)?;

        Ok(Html::from(String::from_utf8(buf)?))
    }

    pub async fn post(Path(slug): Path<String>) -> Result<Html<String>, RouteError> {
        Ok(get_static_file_for_slug(&slug).await?)
    }

    pub async fn posts_list() -> PageResult {
        match blog::make_posts_index() {
            Ok(s) => Ok(Html(s)),
            Err(e) => Err(RouteError::from(e)),
        }
    }

    pub async fn home() -> PageResult {
        Ok(Html("<h1>This is the homepage!</h1>".into()))
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
    Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
