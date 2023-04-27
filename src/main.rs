#![allow(clippy::needless_return)]

use anyhow::Result;
use serde::Deserialize;
use std::{collections::HashMap, fs};
use utils::main::{changes_str, fetch_changes};

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

    let package_json_string = fs::read_to_string(PACKAGE_JSON)?;
    let package_json: PackageJSON = serde_json::from_str(&package_json_string)?;

    let deps_changes = fetch_changes(&package_json.dependencies, &http);
    let dev_deps_changes = fetch_changes(&package_json.dev_dependencies, &http);

    let result = futures_util::future::join_all(vec![deps_changes, dev_deps_changes]).await;
    let deps_changes = result[0].as_ref().unwrap();
    let dev_deps_changes = result[1].as_ref().unwrap();

    println!("Dependencies:\n");
    println!("{}", changes_str(deps_changes));

    println!("DevDependencies:\n");
    println!("{}", changes_str(dev_deps_changes));

    return Ok(());
}
