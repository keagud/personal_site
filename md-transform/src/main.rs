use anyhow;
use rcdom::NodeData;
use regex;
use std::fs;
use std::fs::read_to_string;
use std::fs::File;
use std::path::PathBuf;

use anyhow::format_err;
use html5ever::{
    self, parse_document,
    serialize::{serialize, SerializeOpts},
    tendril::TendrilSink,
    tree_builder::{self, TreeBuilderOpts},
    ParseOpts,
};
use markdown;
use markdown::to_html_with_options;
use markup5ever_rcdom as rcdom;
use markup5ever_rcdom::RcDom;
use scraper::{Html, Selector};

//"\(:sidenote(.*)sidenote:\)"gms
fn parse_html(html_doc: &str) -> anyhow::Result<RcDom> {
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
        .map_err(|e| anyhow::Error::from(e))
}

fn md_file_to_html(md_path: &PathBuf) -> anyhow::Result<String> {
    let file_content = read_to_string(md_path)?;

    to_html_with_options(&file_content, &markdown::Options::gfm()).map_err(|e| format_err!("{}", e))
}

fn process_sidenotes(document_input: &str) -> String {
    let mut document = String::from(document_input);
    let pattern_str = format!(r#"\(:sidenote(?<text>.*?)sidenote:\)"#);
    let mut counter: usize = 1;
    let re = regex::RegexBuilder::new(&pattern_str)
        .dot_matches_new_line(true)
        .build()
        .unwrap();

    while let Some(_) = re.find(&document) {
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

fn walk(indent: usize, handle: &rcdom::Handle) {
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

fn main() -> anyhow::Result<()> {
    const POSTS_MD_DIR: &str = "../assets/posts/md";

    let posts_md_dir = PathBuf::from(POSTS_MD_DIR).canonicalize()?;

    let test_post = posts_md_dir.join("sidenote.md").canonicalize()?;

    let rendered_html = md_file_to_html(&test_post).unwrap();

    let s = process_sidenotes(&rendered_html);

    let f = fs::write(PathBuf::from("../assets/").join("rendered.html"), &s)?;

    println!("{s}");

    Ok(())
}
