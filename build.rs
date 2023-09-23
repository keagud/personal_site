use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::option_env;
use std::path;

fn load_key_from_file() -> Result<String, Box<dyn Error>> {
    let env_contents = fs::read_to_string("./.env")?;
    let key_value = String::from(&env_contents)
        .lines()
        .filter_map(|l| l.split_once("="))
        .collect::<HashMap<&str, &str>>()
        .get("SITE_ADMIN_KEY")
        .ok_or("Not found")?
        .to_string();

    Ok(key_value)
}

fn main() {
    let key = dbg!(load_key_from_file()).unwrap_or_else(|e| {
        let expect_msg = format!(
            "Key should either be in .env or shell environment at build time, got this: {e:?}",
        );
        std::option_env!("SITE_ADMIN_KEY")
            .expect(&expect_msg)
            .to_string()
    });

    println!("cargo:rustc-env=SITE_ADMIN_KEY={key}");
}
