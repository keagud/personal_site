use blog::init_table_connection;
use blog::reply_from_slug;
use warp::Filter;

pub mod common {

    use chrono::{DateTime, NaiveDateTime, Utc};
    pub static POSTS_DB_PATH: &str = "assets/posts.db";
    pub static POSTS_FILES_PATH: &str = "assets/posts/";
    pub static TEMPLATES_PATH: &str = "assets/templates";

    pub fn timestamp_date_format(timestamp: usize, format_str: &str) -> String {
        let naive =
            NaiveDateTime::from_timestamp_opt(timestamp as i64, 0).expect("Timestamp is valid");

        let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

        dt.format(format_str).to_string()
    }
}

#[allow(dead_code)]
pub mod blog {

    use anyhow::{self, format_err};
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    use crate::common;

    use rusqlite;
    use std::fs::read_to_string;
    use warp::reply::{html, with_status};

    #[derive(Debug, PartialEq, Eq)]
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

    pub fn reply_from_slug(slug: &str) -> Box<dyn warp::Reply> {
        let post_file_path_str = format!("{}/{}.html", common::POSTS_FILES_PATH, slug);
        let post_file_path = PathBuf::from(&post_file_path_str);

        if let Ok(s) = read_to_string(post_file_path) {
            Box::new(html(s))
        } else {
            Box::new(with_status(
                html("404 page goes here"),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
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

    pub fn make_posts_list(
        conn: &rusqlite::Connection,
        _format_fn: &dyn Fn(&Post) -> String,
    ) -> Box<dyn warp::Reply> {
        let _all_posts = get_all_post_metadata(conn).expect("Post metadata");
        todo!();
    }
}

pub mod render {

    use std::path::PathBuf;

    use crate::blog;
    use crate::blog::PostDisplay;
    use crate::common;
    use anyhow::format_err;
    use handlebars;
    use handlebars::{to_json, Handlebars};
    use rusqlite::Connection;
    use serde_json;

    struct BasePageParams {
        pub title: String,
        pub content: String,
    }

    pub trait RenderablePage {
        fn title(&self) -> String;
        fn content(&self) -> anyhow::Result<String>;
    }

    #[derive(Debug)]
    pub struct IndexPage {
        pub posts: Vec<blog::PostDisplay>,
    }

    impl IndexPage {
        pub fn from_db(conn: &Connection) -> anyhow::Result<Self> {
            let posts = blog::get_all_post_metadata(conn)?
                .iter()
                .map(|p| p.as_display())
                .collect::<Vec<blog::PostDisplay>>();

            Ok(IndexPage { posts })
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
            let list_items_json = to_json(&self.posts);
            template_values.insert(String::from("posts"), list_items_json);

            let rendered_content = hb.render("posts_list".into(), &template_values)?;
            Ok(rendered_content)
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
    let conn = init_table_connection()?;

    let first_post = blog::Post {
        title: String::from("Shave your head!"),
        timestamp: 1687935600,
        slug: String::from("bald"),
    };

    blog::add_post_metadata_to_db(&conn, &first_post);

    let ip = render::IndexPage::from_db(&conn)?;

    let content = render::render_into_base(&ip)?;
    println!("{ip:?}");
    println!("{content}");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conn = init_table_connection()?;

    let index_page = render::IndexPage::from_db(&conn)?;


    let route = warp::path!("blog" / String).map(|slug: String| reply_from_slug(&slug));

    warp::serve(route).run(([127, 0, 0, 1], 8000)).await;

    Ok(())
}
