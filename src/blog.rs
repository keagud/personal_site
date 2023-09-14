use anyhow::{self, format_err};
use serde::{Deserialize, Serialize};
use std::{
    fs,
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

pub fn md_file_to_html(md_path: &PathBuf) -> anyhow::Result<String> {
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

    while re.find(&document).is_some() {
        let mn_id = format!("mn-{counter}");
        counter += 1;

        let replacement = format!(
            r#"<label for="{mn_id}" class="margin-toggle"> &#8853;</label> 
            <input type="checkbox" id="{mn_id}" class="margin-toggle"/>
            <span class="marginnote">
            $text
            </span> "#
        );

        let rep = re.replace(&document, replacement).to_string();

        document = rep;
    }

    document
}

pub fn walk(indent: usize, handle: &rcdom::Handle) {
    let node = handle;

    for _ in 0..indent {
        print!(" ");
    }

    if let NodeData::Element { ref name, .. } = node.data {
        println!("{:?} ", name,);
    }

    for child in node.children.borrow().iter() {
        walk(indent + 4, child);
    }
}
