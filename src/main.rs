use axum::{routing::get, Router, Server};

mod render;
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

pub mod blog {

    use anyhow::{self, format_err};
    use serde::{Deserialize, Serialize};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use crate::common;

    use handlebars::Handlebars;
    use rusqlite;
    use rusqlite::Connection;
    use serde_json;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Post {
        pub title: String,
        pub timestamp: usize,
        pub slug: String,
    }

    impl Post {
        pub fn date_str(&self) -> String {
            common::timestamp_date_format(self.timestamp, "%F")
        }

        pub fn as_display(&self) -> PostDisplay {
            let date = self.date_str();
            PostDisplay {
                title: self.title.to_owned(),
                slug: self.slug.to_owned(),
                date,
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PostDisplay {
        pub title: String,
        pub date: String,
        pub slug: String,
    }

    pub fn add_post_metadata_to_db(conn: &rusqlite::Connection, post: &Post) -> anyhow::Result<()> {
        let post_files_path = PathBuf::from(common::POSTS_FILES_PATH).canonicalize()?;
        let post_filename = format!("{}.md", post.slug.to_lowercase());

        let resolved_path = post_files_path.join(post_filename);

        if !resolved_path.try_exists()? {
            return Err(format_err!("Invalid path: {:?}", resolved_path));
        }

        let path_str = resolved_path.to_str().ok_or(format_err!(
            "Path cannot be resolved to UTF-8: {:?}",
            resolved_path
        ))?;

        conn.execute(
            r#"INSERT INTO post (title, timestamp, slug, content_path) VALUES (?1, ?2, ?3, ?4)"#,
            (&post.title, &post.timestamp, &post.slug, path_str),
        )?;

        Ok(())
    }

    pub fn init_table_connection() -> anyhow::Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(common::POSTS_DB_PATH)?;

        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS post(
          id INTEGER PRIMARY KEY,
          title VARCHAR(255) NOT NULL,
          timestamp INTEGER NOT NULL,
          slug VARCHAR(255) UNIQUE NOT NULL
        );
        "#,
            (),
        )?;

        Ok(conn)
    }

    pub fn get_all_post_metadata(conn: &rusqlite::Connection) -> anyhow::Result<Vec<Post>> {
        let mut stmt =
            conn.prepare("SELECT title, timestamp, slug FROM post ORDER BY timestamp DESC;")?;

        let posts_iter = stmt.query_map([], |row| {
            Ok(Post {
                title: row.get(0)?,
                timestamp: row.get(1)?,
                slug: row.get(2)?,
            })
        })?;

        Ok(posts_iter.filter_map(|p| p.ok()).collect::<Vec<Post>>())
    }

    fn post_data_as_json_str(conn: &rusqlite::Connection) -> anyhow::Result<String> {
        let post_data = get_all_post_metadata(conn)?;
        Ok(serde_json::to_string_pretty(&post_data)?)
    }

    pub fn dump_posts_json(
        conn: &rusqlite::Connection,
        dump_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let dump_path_buf = PathBuf::from(dump_path.as_ref());

        match dump_path_buf.extension() {
            None => Err(format_err!("Invalid file type: must be json")),
            Some(o) => match o.to_str() {
                None => Err(format_err!("Invalid file name")),
                Some(s) if s == "json" => Ok(()),
                Some(x) => Err(format_err!("Invalid filetype {x:?}; must be json")),
            },
        }?;

        let posts_json_str = post_data_as_json_str(conn)?;
        fs::write(dump_path_buf, posts_json_str)?;

        Ok(())
    }

    pub trait RenderablePage {
        fn title(&self) -> String;
        fn content(&self) -> anyhow::Result<String>;
    }

    #[derive(Debug)]
    pub struct IndexPage(Vec<PostDisplay>);

    impl IndexPage {
        pub fn from_db(conn: &Connection) -> anyhow::Result<Self> {
            let posts = get_all_post_metadata(conn)?
                .iter()
                .map(|p| p.as_display())
                .collect::<Vec<PostDisplay>>();

            Ok(IndexPage(posts))
        }
    }

    impl RenderablePage for IndexPage {
        fn title(&self) -> String {
            String::from("Posts")
        }

        fn content(&self) -> anyhow::Result<String> {
            let mut hb = Handlebars::new();
            let template_path = get_template_path("posts_list")?;

            hb.register_template_file("posts_list", template_path)?;

            let mut template_values = serde_json::Map::new();
            let list_items_json = handlebars::to_json(&self.0);
            template_values.insert(String::from("posts"), list_items_json);

            let rendered_content = hb.render("posts_list", &template_values)?;
            Ok(rendered_content)
        }
    }

    pub struct PostPage(Post);

    impl RenderablePage for PostPage {
        fn title(&self) -> String {
            String::from(&self.0.title)
        }

        fn content(&self) -> anyhow::Result<String> {
            todo!()
        }
    }

    pub fn render_into_base<T: RenderablePage>(page: &T) -> anyhow::Result<String> {
        let base_template_path = get_template_path("base")?;
        let mut hb = Handlebars::new();
        hb.register_template_file("base", base_template_path)?;

        let page_title = page.title();
        let page_body_content = page.content()?;

        let rendered_content = hb.render(
            "base",
            &serde_json::json!({"title" : page_title, "content": page_body_content}),
        )?;
        Ok(rendered_content)
    }

    pub fn make_posts_index() -> anyhow::Result<String> {
        let conn = init_table_connection()?;
        let index_page = IndexPage::from_db(&conn)?;

        let raw = render_into_base(&index_page)?;

        Ok(raw)
    }

    fn get_template_path(template_name: &str) -> anyhow::Result<PathBuf> {
        let p = PathBuf::from(common::TEMPLATES_PATH);
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

    pub fn init_templates(template_names: Vec<&'static str>) -> Handlebars {
        let mut hb = Handlebars::new();

        for t in template_names {
            let template_path = get_template_path(t).expect("Template should be valid");
            hb.register_template_file(t, template_path)
                .expect("Template registration ok");
        }

        hb
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
