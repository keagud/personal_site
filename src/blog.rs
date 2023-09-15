use anyhow::{self, format_err};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::common;

use std::fs::read_to_string;

use handlebars::Handlebars;
use rusqlite;
use rusqlite::Connection;
use serde_json;

use html5ever::{
    self, parse_document, tendril::TendrilSink, tree_builder::TreeBuilderOpts, ParseOpts,
};
use markdown;
use markdown::to_html_with_options;
use markup5ever_rcdom as rcdom;
use markup5ever_rcdom::RcDom;

use rcdom::NodeData;

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

    pub fn md_path(&self) -> PathBuf {
        PathBuf::from(common::POSTS_MARKDOWN_PATH).join(format!("{}.md", self.slug))
    }

    pub fn html_path(&self) -> PathBuf {
        PathBuf::from(common::POSTS_FILES_PATH).join(format!("{}.html", self.slug))
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
            r#"INSERT OR IGNORE INTO post (title, timestamp, slug) VALUES (?1, ?2, ?3, )"#,
            (&post.title, &post.timestamp, &post.slug),
        )?;
    }

    Ok(())
}

pub fn posts_index_from_db(conn: &Connection) -> anyhow::Result<Vec<PostDisplay>> {
    let posts = get_all_post_metadata(conn)?
        .iter()
        .map(|p| p.as_display())
        .collect::<Vec<PostDisplay>>();

    Ok(posts)
}

pub fn post_index_display(posts: &Vec<PostDisplay>) -> anyhow::Result<String> {
    let mut hb = Handlebars::new();
    let template_path = get_template_path("posts_list")?;

    hb.register_template_file("posts_list", template_path)?;

    let mut template_values = serde_json::Map::new();
    let list_items_json = handlebars::to_json(posts);
    template_values.insert(String::from("posts"), list_items_json);

    let rendered_content = hb.render("posts_list", &template_values)?;
    Ok(rendered_content)
}

pub fn make_posts_index() -> anyhow::Result<String> {
    let conn = init_table_connection()?;
    let posts = posts_index_from_db(&conn)?;
    post_index_display(&posts)
}

pub enum Page {
    HomePage,
    PostPage(Post),
    PostListPage(Vec<PostDisplay>),
}

impl Page {
    pub fn title(&self) -> String {
        match self {
            Self::HomePage => "Home".into(),
            Self::PostPage(p) => p.title.clone(),
            Self::PostListPage(_) => "Index of posts".into(),
        }
    }

    pub fn render_content(&self) -> anyhow::Result<String> {
        match self {
            Self::HomePage => {
                let homepage_path = PathBuf::from(common::STATIC_PAGES_PATH)
                    .join("home.html")
                    .canonicalize()?;
                md_file_to_html(homepage_path)
            }

            Self::PostPage(p) => md_file_to_html(&p.md_path()),

            Self::PostListPage(ps) => post_index_display(ps),
        }
    }

    pub fn render_into_base(&self) -> anyhow::Result<String> {
        let base_template_path = get_template_path("base")?;
        let mut hb = Handlebars::new();
        hb.register_template_file("base", base_template_path)?;

        let page_title = self.title();
        let page_body_content = self.render_content()?;

        let rendered_content = hb.render(
            "base",
            &serde_json::json!({"title" : page_title, "content": page_body_content}),
        )?;
        Ok(rendered_content)
    }
}

pub fn render_into_base(page: Page) -> anyhow::Result<String> {
    let base_template_path = get_template_path("base")?;
    let mut hb = Handlebars::new();
    hb.register_template_file("base", base_template_path)?;

    let page_title = page.title();
    let page_body_content = page.render_content()?;

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

pub fn parse_html(html_doc: &str) -> anyhow::Result<RcDom> {
    let tb = TreeBuilderOpts {
        scripting_enabled: false,
        ..TreeBuilderOpts::default()
    };
    let opts = ParseOpts {
        tree_builder: tb,
        ..Default::default()
    };

    parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut html_doc.as_bytes())
        .map_err(anyhow::Error::from)
}

pub fn md_file_to_html(md_path: impl AsRef<Path>) -> anyhow::Result<String> {
    let file_content = read_to_string(md_path)?;

    to_html_with_options(&file_content, &markdown::Options::gfm()).map_err(|e| format_err!("{}", e))
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
