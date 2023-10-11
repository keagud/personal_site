use chrono::{DateTime, NaiveDateTime, Utc};
use common::asset;
use handlebars::Handlebars;
use serde_json::json;
use std::sync::Arc;
use tide::{self, StatusCode, http::mime};

pub mod blog;
use blog::db;

macro_rules! crate_root {
    ($s:literal) => {
        concat!(std::env!("CARGO_MANIFEST_DIR", $s))
    };
}

const BASE_TEMPLATE_PATH: &'static str = crate_root!("/assets/templates/base.html");
const POST_LIST_TEMPLATE_PATH: &'static str = crate_root!("/assets/templates/posts_list.html");

type PostsDb = Arc<sqlx::Pool<sqlx::Sqlite>>;

pub fn timestamp_date_format(timestamp: i64, format_str: &str) -> String {
    let naive = NaiveDateTime::from_timestamp_opt(timestamp, 0).expect("Timestamp is valid");

    let dt: DateTime<Utc> = naive.and_local_timezone(Utc).unwrap();

    dt.format(format_str).to_string()
}
pub struct Routes<'a> {
    hb: Box<Handlebars<'a>>,
}

impl<'a> Routes<'a> {
    pub fn new() -> Self {
        let mut hb = Handlebars::new();

        hb.register_template_file("base", BASE_TEMPLATE_PATH)
            .unwrap();

        hb.register_template_file("posts_list", POST_LIST_TEMPLATE_PATH)
            .unwrap();
        Self { hb: Box::new(hb) }
    }

    pub async fn posts_list_page(&self, req: tide::Request<PostsDb>) -> tide::Result {
        let conn = req.state();
        let all_posts = db::fetch_all_post_metadata(&conn)
            .await
            .map_err(|e| tide::Error::new(StatusCode::InternalServerError, e))?;

        let template_json: Vec<serde_json::Value> = all_posts
            .iter()
            .map(|p| 
                json!( {"date" : timestamp_date_format(p.timestamp, "%F"), "slug" : p.slug, "title": p.title})).collect();

        let rendered = self.hb.render("posts_list", &template_json)?;

        //TODO render into the base template as well
        let res = tide::Response::builder(StatusCode::Ok).content_type(mime::HTML).body(rendered).build();

        Ok(res)
    }

    pub async fn post_page() -> tide::Response {
        todo!();
    }

    pub async fn homepage() -> tide::Response {
        todo!();
    }

    pub async fn about_page() -> tide::Response {
        todo!();
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let conn = Arc::new(
        db::init_db()
            .await
            .expect("Database initialization shouldn't fail"),
    );

    let mut app = tide::with_state(conn);

    app.at("/").serve_file(crate_root!("/static/homepage.html"));
    app.at("/about")
        .serve_file(crate_root!("/static/about.html"));
    app.listen("0.0.0:8000").await?;
    Ok(())
}
