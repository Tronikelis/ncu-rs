#![allow(clippy::needless_return)]

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs};
use utils::main::{changes_str, fetch_changes, replace_deps};

mod utils;

const PACKAGE_JSON: &str = "./package.json";

#[derive(Deserialize, Debug)]
struct PackageJSON {
    #[serde(rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
    dependencies: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let http = reqwest::Client::new();

    let package_json_string = fs::read_to_string(PACKAGE_JSON)
        .map_err(|_| anyhow!("package.json not found, are you running the cli in the same dir?"))?;

    let package_json: PackageJSON = serde_json::from_str(&package_json_string)?;

    let deps_changes = fetch_changes(&package_json.dependencies, &http).await?;
    let dev_deps_changes = fetch_changes(&package_json.dev_dependencies, &http).await?;

    println!("\n\nDependencies:\n");
    println!("{}", changes_str(&deps_changes));

    println!("DevDependencies:\n");
    println!("{}", changes_str(&dev_deps_changes));

    replace_deps(PACKAGE_JSON, &deps_changes)?;
    replace_deps(PACKAGE_JSON, &dev_deps_changes)?;

    return Ok(());
}
