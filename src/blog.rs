use anyhow::{self, format_err};

use std::path::PathBuf;

use crate::common;

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
                .map_err(|e| anyhow::Error::from(e))
        }
    }

    pub fn add_post_metadata_to_db(conn: &rusqlite::Connection, post: &Post) -> anyhow::Result<()> {
        let post_files_path = PathBuf::from(common::POSTS_FILES_PATH).canonicalize()?;
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
    use markdown;
    use markdown::to_html_with_options;
    use serde::{Deserialize, Serialize};
    use std::fs::{self, read_to_string, File};
    use std::io::Read;
    use std::path::{Path, PathBuf};

    #[derive(Deserialize, Serialize)]
    struct RenderParams {
        pub title: String,
        pub home_url: String,
        pub content: String,
        pub favicon_path: String,
        pub quotes_list_json: String,
        pub css: String,
    }

    pub const FAVICON_URL: &str = "/static/favicon.io";
    pub const CSS_PATH: &str = "assets/style.css";
    pub const QUOTES_PATH: &str = "assets/quotes.json";

    //read from assets/quotes.json
    fn load_quotes() -> String {
        read_to_string(QUOTES_PATH).expect("Hardcoded json path should work")
    }

    //load css as a string
    fn load_css() -> String {
        read_to_string(CSS_PATH).expect("Hardcoded CSS path should work")
    }

    impl Default for RenderParams {
        fn default() -> Self {
            RenderParams {
                title: "Welcome to my web site!".into(),
                home_url: "/".into(),
                content: String::new(),
                favicon_path: FAVICON_URL.into(),
                quotes_list_json: load_quotes(),
                css: load_css(),
            }
        }
    }

    impl RenderParams {
        pub fn new(title: &str, content: &str) -> Self {
            Self {
                title: String::from(title),
                content: String::from(content),
                ..Self::default()
            }
        }
    }

    pub fn render_md(page_title: &str, md_file: impl AsRef<Path>) -> anyhow::Result<String> {
        md_file_to_html(md_file).and_then(|ref s| render_html_str(page_title, s))
    }

    pub fn render_html_str(page_title: &str, page_content: &str) -> anyhow::Result<String> {
        render_into_base(page_title, &process_sidenotes(page_content))
    }

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

    pub fn render_into_base(title: &str, content: &str) -> anyhow::Result<String> {
        let base_template_path = get_template_path("base")?;

        let render_params = RenderParams::new(title, content);

        let mut hb = Handlebars::new();
        hb.register_template_file("base", base_template_path)?;

        let rendered_content = hb.render("base", &serde_json::to_value(render_params)?)?;
        Ok(rendered_content)
    }

    fn md_file_to_html(md_path: impl AsRef<Path>) -> anyhow::Result<String> {
        let file_content = read_file_contents(md_path)?;

        to_html_with_options(&file_content, &markdown::Options::gfm())
            .map_err(|e| format_err!("{}", e))
    }

    pub fn process_sidenotes(document_input: &str) -> String {
        let mut document = String::from(document_input);
        let pattern_str = r"\(:sidenote(?<text>.*?)sidenote:\)".to_string();
        let mut counter: usize = 1;
        let re = regex::RegexBuilder::new(&pattern_str)
            .dot_matches_new_line(true)
            .build()
            .unwrap();

        while re.is_match(&document) {
            let mn_id = format!("mn-{counter}");
            counter += 1;

            let replacement = format!(
                r#"<label for="{mn_id}" class="margin-toggle"> &#8853;</label> 
            <input type="checkbox" id="{mn_id}" class="margin-toggle"/>
            <span class="marginnote">
            $text
            </span> "#
            );

            document = re.replace(&document, replacement).to_string();
        }

        document
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
