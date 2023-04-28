#![allow(clippy::needless_return)]

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs};

mod options;
use options::main::Options;

mod utils;
use utils::main::{changes_str, fetch_changes, replace_deps};

const PACKAGE_JSON: &str = "./package.json";

#[derive(Deserialize, Debug)]
struct PackageJSON {
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    dependencies: Option<HashMap<String, String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::new()?;
    let http = reqwest::Client::new();

    let package_json_string = fs::read_to_string(PACKAGE_JSON)
        .map_err(|_| anyhow!("package.json not found, are you running the cli in the same dir?"))?;

    let package_json: PackageJSON = serde_json::from_str(&package_json_string)?;

    let deps_changes = match package_json.dependencies {
        Some(dependencies) => fetch_changes(&dependencies, &http, &options).await?,
        None => vec![],
    };
    let dev_deps_changes = match package_json.dev_dependencies {
        Some(dependencies) => fetch_changes(&dependencies, &http, &options).await?,
        None => vec![],
    };

    println!("\n\nDependencies:\n");
    println!("{}", changes_str(&deps_changes));

    println!("DevDependencies:\n");
    println!("{}", changes_str(&dev_deps_changes));

    if !options.write {
        println!("-w to write changes to package.json");
    }

    if options.write {
        replace_deps(PACKAGE_JSON, &deps_changes)?;
        replace_deps(PACKAGE_JSON, &dev_deps_changes)?;
    }

    return Ok(());
}
