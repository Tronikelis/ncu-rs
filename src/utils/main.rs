use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PkgChange {
    pub from: String,
    pub to: String,
    pub pkg: String,
}

#[derive(Deserialize, Debug)]
struct DistTags {
    latest: String,
}

#[derive(Deserialize)]
struct RegistryGet {
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
}

pub async fn fetch_changes(
    deps: &HashMap<String, String>,
    http: &Client,
) -> Result<Vec<PkgChange>> {
    let mut changes: Vec<PkgChange> = vec![];

    for (pkg, pkg_version) in deps {
        let response = http
            .get(format!("https://registry.npmjs.com/{}", pkg))
            .send()
            .await?
            .text()
            .await?;

        let registry_get: RegistryGet = serde_json::from_str(&response)?;

        if pkg_version.clone() != registry_get.dist_tags.latest {
            changes.push(PkgChange {
                from: pkg_version.clone(),
                to: registry_get.dist_tags.latest,
                pkg: pkg.clone(),
            });
        }
    }

    return Ok(changes);
}

pub fn changes_str(pkg_changes: &Vec<PkgChange>) -> String {
    let mut changes = String::new();

    for change in pkg_changes {
        let string = format!("{}: {} => {}\n", change.pkg, change.from, change.to);
        changes.push_str(&string);
    }

    return changes;
}
