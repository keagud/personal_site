pub mod db {

    use crate::common;
    use anyhow::format_err;
    use common::Post;
    use std::fs::File;
    use std::io::BufReader;

    use anyhow;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    pub struct DbConnection {
        pub conn: rusqlite::Connection,
    }

    impl Drop for DbConnection {
        fn drop(&mut self) {
            match self.dump_json(common::POSTS_JSON_PATH) {
                Ok(_) => (),
                Err(e) => panic!("Cannot dump db content to json: {:?}", e),
            }
        }
    }

    impl DbConnection {
        pub fn new() -> anyhow::Result<Self> {
            Ok(DbConnection {
                conn: init_table_connection()?,
            })
        }

        pub fn add_post_data(&mut self, post: &Post) -> anyhow::Result<()> {
            add_post_metadata_to_db(&self.conn, post)
        }

        pub fn all_posts(&self) -> anyhow::Result<Vec<Post>> {
            get_all_post_metadata(&self.conn)
        }

        pub fn dump_json(&self, json_path: impl AsRef<Path>) -> anyhow::Result<()> {
            dump_posts_json(&self.conn, json_path)
        }

        pub fn get(&self, slug: &str) -> anyhow::Result<Post> {
            let stmt = format!("SELECT title, timestamp, slug FROM post WHERE slug='{slug}'");

            self.conn
                .query_row(&stmt, [], |row| {
                    Ok(Post {
                        title: row.get(0)?,
                        timestamp: row.get(1)?,
                        slug: row.get(2)?,
                    })
                })
                .map_err(anyhow::Error::from)
        }
    }

    pub fn add_post_metadata_to_db(conn: &rusqlite::Connection, post: &Post) -> anyhow::Result<()> {
        let post_files_path = PathBuf::from(common::POSTS_MARKDOWN_PATH).canonicalize()?;
        let post_filename = format!("{}.md", post.slug.to_lowercase());

        let resolved_path = post_files_path.join(post_filename);

        if !resolved_path.try_exists()? {
            return Err(format_err!(
                "Content not found at path: {:?}",
                resolved_path
            ));
        }

        conn.execute(
            r#"INSERT INTO post (title, timestamp, slug) VALUES (?1, ?2, ?3)"#,
            (&post.title, &post.timestamp, &post.slug),
        )?;

        Ok(())
    }

    fn init_table_connection() -> anyhow::Result<rusqlite::Connection> {
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

        load_posts_json(&conn, common::POSTS_JSON_PATH)?;

        Ok(conn)
    }

    fn get_all_post_metadata(conn: &rusqlite::Connection) -> anyhow::Result<Vec<Post>> {
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

    fn assure_is_json_path(p: &impl AsRef<Path>) -> anyhow::Result<()> {
        match p.as_ref().extension() {
            None => Err(format_err!("Invalid file type: must be json")),
            Some(o) => match o.to_str() {
                None => Err(format_err!("Invalid file name")),
                Some(s) if s == "json" => Ok(()),
                Some(x) => Err(format_err!("Invalid filetype {x:?}; must be json")),
            },
        }
    }

    pub fn dump_posts_json(
        conn: &rusqlite::Connection,
        dump_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let dump_path_buf = PathBuf::from(dump_path.as_ref());

        assure_is_json_path(&dump_path_buf)?;
        let posts_json_str = post_data_as_json_str(conn)?;
        fs::write(dump_path_buf, posts_json_str)?;

        Ok(())
    }

    pub fn load_posts_json(
        conn: &rusqlite::Connection,
        load_path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        assure_is_json_path(&load_path)?;

        let fp = File::open(load_path)?;

        let reader = BufReader::new(fp);
        let posts: Vec<Post> = serde_json::from_reader(reader)?;

        for ref post in posts {
            conn.execute(
                r#"INSERT OR REPLACE INTO post (title, timestamp, slug) VALUES (?1, ?2, ?3);"#,
                (&post.title, &post.timestamp, &post.slug),
            )?;
        }

        Ok(())
    }
}

pub mod render {
    use crate::common;
    use crate::common::Post;
    use anyhow;
    use anyhow::format_err;
    use handlebars::Handlebars;

    use std::fs::File;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    pub use md_render::*;

    pub fn read_file_contents(file_path: impl AsRef<Path>) -> anyhow::Result<String> {
        let file_path = PathBuf::from(file_path.as_ref());

        let mut file_handle = match file_path.metadata() {
            Err(e) => Err(anyhow::Error::from(e)),
            Ok(m) if m.is_dir() => Err(format_err!("bad")),
            Ok(_) => match File::open(file_path) {
                Ok(f) => Ok(f),
                Err(e) => Err(anyhow::Error::from(e)),
            },
        }?;

        let mut buf: Vec<u8> = Vec::new();
        file_handle.read_to_end(&mut buf)?;

        Ok(String::from_utf8(buf)?)
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

    pub fn post_index_display(posts: &Vec<Post>) -> anyhow::Result<String> {
        let mut hb = Handlebars::new();
        let template_path = get_template_path("posts_list")?;

        hb.register_template_file("posts_list", template_path)?;

        let mut template_values = serde_json::Map::new();
        let list_items_json = handlebars::to_json(posts);
        template_values.insert(String::from("posts"), list_items_json);

        let rendered_content = hb.render("posts_list", &template_values)?;

        Ok(dbg!(rendered_content))
    }
}
