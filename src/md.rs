use anyhow;
use anyhow::format_err;
use handlebars::Handlebars;
use markdown;
use markdown::to_html_with_options;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::include_str;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
struct RenderParams {
    pub title: String,
    pub home_url: String,
    pub content: String,
    pub favicon_path: String,
    pub quotes_list_json: &'static str,
    pub css: &'static str,
}

pub const FAVICON_URL: &str = "/static/favicon.io";
static CSS: &str = include_str!("../assets/style.css");
static QUOTES: &str = include_str!("../assets/quotes.json");

static BASE_TEMPLATE: &str = include_str!("../assets/templates/base.html");

impl Default for RenderParams {
    fn default() -> Self {
        RenderParams {
            title: "Welcome to my web site!".into(),
            home_url: "/".into(),
            content: String::new(),
            favicon_path: FAVICON_URL.into(),
            quotes_list_json: QUOTES,
            css: CSS,
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

#[derive(Default)]
pub struct RenderBuilder {
    title: String,
    md_content: Option<String>,
    html_content: Option<String>,
    sidenotes: bool,
    into_base_template: bool,
}

impl RenderBuilder {
    pub fn new(title: &str) -> Self {
        RenderBuilder {
            title: title.into(),
            ..Self::default()
        }
    }
    pub fn render(&self) -> anyhow::Result<String> {
        //if html was given directly, use that,
        //Otherwise get the string markdown content to render
        let mut html_str = if let Some(ref html_content) = self.html_content {
            if self.md_content.is_some() {
                Err(format_err!("Ambiguous"))
            } else {
                Ok(html_content.clone())
            }
        } else {
            let md_content = match &self.md_content {
                Some(c) => Ok(c),
                None => Err(format_err!("No content")),
            }?;

            //render it to html
            to_html_with_options(md_content, &markdown::Options::gfm())
                .map_err(|e| format_err!("{}", e))
        }?;

        //if applicable, do postprocessing

        if self.sidenotes {
            html_str = process_sidenotes(&html_str);
        }

        if self.into_base_template {
            let mut hb = Handlebars::new();

            hb.register_template_string("base", BASE_TEMPLATE)?;

            let render_params = RenderParams::new(&self.title, &html_str);

            html_str = hb.render("base", &serde_json::to_value(render_params)?)?;
        }

        Ok(html_str)
    }

    pub fn html_content<'a>(&'a mut self, html_content: &str) -> &'a mut Self {
        self.html_content = Some(html_content.into());
        self
    }

    pub fn md_content<'a>(&'a mut self, content: &str) -> &'a mut Self {
        self.md_content = Some(content.into());
        self
    }

    pub fn sidenotes(&mut self) -> &mut Self {
        self.sidenotes = true;
        self
    }

    pub fn into_base_template(&mut self) -> &mut Self {
        self.into_base_template = true;
        self
    }
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
