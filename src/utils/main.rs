use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
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

#[derive(Clone)]
struct Pkg {
    pkg: String,
    version: String,
}

pub async fn fetch_changes(
    deps: &HashMap<String, String>,
    http: &Client,
) -> Result<Vec<PkgChange>> {
    let mut handles: Vec<_> = vec![];

    let pkg_vec: Vec<Pkg> = deps
        .iter()
        .map(|(key, value)| Pkg {
            pkg: key.clone(),
            version: value.clone(),
        })
        .collect();
    let pkg_vec: Arc<Mutex<_>> = Arc::new(Mutex::new(pkg_vec));

    let http = Arc::new(http.clone());

    for _ in 0..5 {
        let handle: JoinHandle<Vec<PkgChange>> = tokio::spawn({
            let http = Arc::clone(&http);
            let pkg_vec = Arc::clone(&pkg_vec);

            let mut changes: Vec<PkgChange> = vec![];

            async move {
                while (*pkg_vec.lock().unwrap()).len() > 0 {
                    let caught_pkg = {
                        let pkg = (*pkg_vec.lock().unwrap()).last().unwrap().clone();
                        (*pkg_vec.lock().unwrap()).pop();
                        pkg
                    };

                    println!("Fetching {}", caught_pkg.pkg);

                    let response = http
                        .get(format!("https://registry.npmjs.com/{}", caught_pkg.pkg))
                        .send()
                        .await
                        .unwrap()
                        .text()
                        .await
                        .unwrap();

                    let registry_get: RegistryGet = serde_json::from_str(&response).unwrap();
                    let latest_registry = registry_get.dist_tags.latest;

                    if caught_pkg.pkg != latest_registry {
                        changes.push(PkgChange {
                            from: caught_pkg.version,
                            to: latest_registry,
                            pkg: caught_pkg.pkg,
                        })
                    }
                }

                return changes;
            }
        });

        handles.push(handle);
    }

    let results: Vec<PkgChange> = futures_util::future::join_all(handles)
        .await
        .iter()
        .map(|value| value.as_ref().unwrap())
        .flatten()
        .map(|value| value.clone())
        .collect();

    return Ok(results);
}

pub fn changes_str(pkg_changes: &Vec<PkgChange>) -> String {
    let mut changes = String::new();

    for change in pkg_changes {
        let string = format!("{}: {} => {}\n", change.pkg, change.from, change.to);
        changes.push_str(&string);
    }

    return changes;
}
