use anyhow;
use rcdom::NodeData;
use std::fs::read_to_string;
use std::path::PathBuf;

use scraper::{Html, Selector};

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

    let test_post = posts_md_dir.join("bald.md").canonicalize()?;

    let rendered_html = md_file_to_html(&test_post).unwrap();

    let dom = parse_html(&rendered_html)?;

    walk(0, &dom.document);

    Ok(())
}
