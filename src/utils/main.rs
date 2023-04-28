use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{to_string_pretty, Map, Value};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct PkgChange {
    pub to: String,
    pub pkg: Pkg,
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

#[derive(Debug, Clone)]
pub struct Pkg {
    pub name: String,
    pub version: String,
    pub prefix: Option<char>,
}

impl Pkg {
    pub fn with_prefix(&self, version: String) -> String {
        match self.prefix {
            Some(prefix) => format!("{}{}", prefix, version),
            None => version,
        }
    }

    pub fn with_prefix_own(&self) -> String {
        match self.prefix {
            Some(prefix) => format!("{}{}", prefix, self.version),
            None => self.version.clone(),
        }
    }
}

// deal with ^,~
fn trim_semver(version: String) -> String {
    let first_char: String = version.chars().next().unwrap().into();
    if first_char.parse::<usize>().is_err() {
        let without_first: String = version.chars().skip(1).collect();
        return without_first;
    }

    return version;
}

pub async fn fetch_changes(
    deps: &HashMap<String, String>,
    http: &Client,
) -> Result<Vec<PkgChange>> {
    let mut handles: Vec<_> = vec![];

    let pkg_vec: Vec<Pkg> = deps
        .iter()
        .map(|(key, value)| {
            let trimmed = trim_semver(value.clone());
            let mut prefix: Option<char> = None;

            if trimmed != *value {
                prefix = Some(value.chars().next().unwrap());
            }

            return Pkg {
                name: key.clone(),
                version: trimmed,
                prefix,
            };
        })
        // skip workspace packages or any others that don't start with a number
        .filter(|pkg| {
            let first_char: String = pkg.version.chars().next().unwrap().into();
            return first_char.parse::<usize>().is_ok();
        })
        .collect();
    let pkg_vec: Arc<Mutex<_>> = Arc::new(Mutex::new(pkg_vec));

    let http = Arc::new(http.clone());

    for _ in 0..10 {
        let handle: JoinHandle<Vec<PkgChange>> = tokio::spawn({
            let http = Arc::clone(&http);
            let pkg_vec = Arc::clone(&pkg_vec);

            let mut changes: Vec<PkgChange> = vec![];

            async move {
                while !(*pkg_vec.lock().unwrap()).is_empty() {
                    let caught_pkg = {
                        let pkg = (*pkg_vec.lock().unwrap()).last().unwrap().clone();
                        (*pkg_vec.lock().unwrap()).pop();
                        pkg
                    };

                    println!("Fetching {}", caught_pkg.name);

                    let response = http
                        .get(format!("https://registry.npmjs.com/{}", caught_pkg.name))
                        .send()
                        .await
                        .unwrap()
                        .text()
                        .await
                        .unwrap();

                    let registry_get: RegistryGet = serde_json::from_str(&response).unwrap();
                    let latest_version = registry_get.dist_tags.latest;

                    if caught_pkg.version != latest_version {
                        changes.push(PkgChange {
                            to: latest_version,
                            pkg: caught_pkg,
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
        .flat_map(|value| value.as_ref().unwrap())
        .cloned()
        .collect();

    return Ok(results);
}

pub fn changes_str(pkg_changes: &Vec<PkgChange>) -> String {
    fn highest_chars(strings: Vec<String>) -> Option<usize> {
        return strings.iter().map(|string| string.chars().count()).max();
    }
    fn whitespace_needed(a: usize, b: usize) -> String {
        let diff = a - b;
        return " ".repeat(diff);
    }

    let mut changes = String::new();

    let name_highest_chars =
        highest_chars(pkg_changes.iter().map(|x| x.pkg.name.clone()).collect()).unwrap_or(20);

    let version_highest_chars = highest_chars(
        pkg_changes
            .iter()
            .map(|x| x.pkg.with_prefix_own())
            .collect(),
    )
    .unwrap_or(10);

    for change in pkg_changes {
        let name_len = change.pkg.name.chars().count();
        let version_len = change.pkg.with_prefix_own().chars().count();

        let whitespace_name = whitespace_needed(name_highest_chars + 10, name_len);
        let whitespace_version = whitespace_needed(version_highest_chars, version_len);

        let string = format!(
            "{}:{}{}{} => {}\n",
            change.pkg.name,
            whitespace_name,
            change.pkg.with_prefix_own(),
            whitespace_version,
            change.pkg.with_prefix(change.to.clone()),
        );
        changes.push_str(&string);
    }

    return changes;
}

pub fn replace_deps(path: &str, changes: &Vec<PkgChange>) -> Result<()> {
    fn replace(dependencies: &mut Map<String, Value>, changes: &Vec<PkgChange>) {
        for (name, version) in dependencies {
            for change in changes {
                if *name != change.pkg.name {
                    continue;
                }

                *version = Value::from(change.pkg.with_prefix(change.to.clone()));
            }
        }
    }

    let mut package_json_raw: Value = serde_json::from_str(&fs::read_to_string(path)?)?;

    let dependencies = package_json_raw["dependencies"].as_object_mut();
    if let Some(x) = dependencies {
        replace(x, changes);
    }

    let dev_dependencies = package_json_raw["devDependencies"].as_object_mut();
    if let Some(x) = dev_dependencies {
        replace(x, changes);
    }

    let overrides = package_json_raw["overrides"].as_object_mut();
    if let Some(x) = overrides {
        replace(x, changes);
    }

    fs::write(path, to_string_pretty(&package_json_raw)?)?;

    return Ok(());
}
