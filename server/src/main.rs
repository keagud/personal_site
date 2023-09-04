use blog::init_table_connection;
use blog::reply_from_slug;
use warp::Filter;

pub mod common {

    pub static POSTS_DB_PATH: &str = "assets/posts.db";
    pub static POSTS_FILES_PATH: &str = "assets/posts/";
}

pub mod blog {

    use anyhow::{self, format_err};
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

    pub fn add_post(conn: &rusqlite::Connection, post: &Post) -> anyhow::Result<()> {
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
          slug VARCHAR(255) UNIQUE NOT NULL,
          content_path VARCHAR(255) NOT NULL
        );
        "#,
            (),
        )?;

        Ok(conn)
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _conn = init_table_connection()?;

    let _first_post = blog::Post {
        title: String::from("Shave your head!"),
        timestamp: 1687935600,
        slug: String::from("bald"),
    };

    let route = warp::path!("blog" / String).map(|slug: String| reply_from_slug(&slug));

    warp::serve(route).run(([127, 0, 0, 1], 8000)).await;

    Ok(())
}
