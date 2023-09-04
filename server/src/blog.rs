#![allow(dead_code)]
use chrono::{DateTime, Utc};
use std::path::PathBuf;

use rusqlite;

pub mod common {
    use std::path::{Path, PathBuf};

    pub static POSTS_DB_PATH: &str = "assets/posts.db";
    pub static POSTS_FILES_PATH: &str = "assets/posts/";
}

#[derive(Debug, PartialEq, Eq)]
pub struct Post {
    title: String,
    date: DateTime<Utc>,
    slug: String,
    content_path: PathBuf,
}
