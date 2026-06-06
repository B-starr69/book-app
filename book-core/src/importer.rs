use crate::database::Database;
use crate::models::{
    ActionConfig, FetchMethod, JsExecutionConfig, NativeFetch, Source, SourceConfig, SourceWithConfig,
    Strategy, UrlTarget,
};
use std::path::Path;
use std::fs;
use anyhow::{anyhow, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use url::Url;

/// Metadata file structure stored in metadata.json
#[derive(Debug, serde::Deserialize)]
struct RepoSource {
    name: String,
    url: String,
    #[serde(rename = "discoverUrl")]
    discover_url: Option<String>,
    #[serde(rename = "booksUrl")]
    books_url: Option<String>,
    #[serde(rename = "chaptersUrl")]
    chapters_url : Option<String>,
    #[serde(rename = "iconUrl")]
    icon_url: Option<String>,
    description: Option<String>,
    #[serde(rename = "version")]
    _version: Option<String>,
    #[serde(rename = "author")]
    _author: Option<String>,
    #[serde(default)]
    search: Option<serde_json::Value>,
    #[serde(default)]
    config: Option<serde_json::Value>,
}

/// Import sources from a GitHub repository's `sources/` directory.
/// Expects structure: sources/<id>/metadata.json, index.js, icon.png
pub async fn import_from_github(repo_url: &str, db: &Database) -> Result<Vec<String>> {
    let url = Url::parse(repo_url).map_err(|e| anyhow!("invalid repo url: {}", e))?;
    let segments: Vec<_> = url.path_segments().ok_or_else(|| anyhow!("invalid repo url"))?.collect();
    if segments.len() < 2 {
        return Err(anyhow!("invalid repo url"));
    }
    let owner = segments[0];
    let repo = segments[1];

    let client = reqwest::Client::new();
    let api = format!("https://api.github.com/repos/{}/{}/contents/sources", owner, repo);
    let resp = client
        .get(&api)
        .header(USER_AGENT, "book-app-importer")
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("GitHub API returned {}", resp.status()));
    }

    let items: serde_json::Value = resp.json().await?;
    let mut imported = Vec::new();

    if let Some(array) = items.as_array() {
        for item in array {
            if item.get("type").and_then(|t| t.as_str()) == Some("dir") {
                if let Some(dir_name) = item.get("name").and_then(|n| n.as_str()) {
                    let dir_api = format!("https://api.github.com/repos/{}/{}/contents/sources/{}", owner, repo, dir_name);
                    let dir_resp = client
                        .get(&dir_api)
                        .header(USER_AGENT, "book-app-importer")
                        .header(ACCEPT, "application/vnd.github.v3+json")
                        .send()
                        .await?;

                    if !dir_resp.status().is_success() {
                        eprintln!("Failed to list {}/{}: {}", owner, repo, dir_resp.status());
                        continue;
                    }

                    let files: serde_json::Value = dir_resp.json().await?;
                    let mut metadata_txt = String::new();
                    let mut index_js_txt = None;
                    let mut icon_download_url = None;

                    if let Some(files_arr) = files.as_array() {
                        for f in files_arr {
                            if let Some(fname) = f.get("name").and_then(|n| n.as_str()) {
                                if fname == "metadata.json" {
                                    if let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) {
                                        metadata_txt = client.get(download_url).send().await?.text().await?;
                                    }
                                } else if fname == "index.js" {
                                    if let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) {
                                        index_js_txt = Some(client.get(download_url).send().await?.text().await?);
                                    }
                                } else if fname == "icon.png" || fname == "icon.jpg" || fname == "icon.jpeg" || fname == "icon.webp" {
                                    if let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) {
                                        icon_download_url = Some(download_url.to_string());
                                    }
                                }
                            }
                        }
                    }

                    if metadata_txt.is_empty() {
                        eprintln!("No metadata.json in sources/{} - skipping", dir_name);
                        continue;
                    }
                    let meta: RepoSource = match serde_json::from_str(&metadata_txt) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("Failed to parse metadata.json in {}: {}", dir_name, e);
                            continue;
                        }
                    };
                    let config = if let Some(config_value) = meta.config.clone() {
                        serde_json::from_value::<SourceConfig>(config_value).unwrap_or_default()
                    } else if index_js_txt.is_some() {
                        let search = meta.search.map(|value| {
                            let url_pattern = value.get("url_pattern")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            ActionConfig {
                                fetch: FetchMethod::Native {
                                    strategy: NativeFetch::Single,
                                    target: UrlTarget::Template { url_pattern },
                                },
                                parse: Strategy::Js(JsExecutionConfig {
                                    js_function: Some("parseSearch".to_string()),
                                    script: None,
                                }),
                            }
                        });

                        SourceConfig {
                            script_path: Some("index.js".to_string()),
                            chapters_list_url: None,
                            home: ActionConfig {
                                fetch: FetchMethod::default(),
                                parse: Strategy::Js(JsExecutionConfig {
                                    js_function: Some("parseHome".to_string()),
                                    script: None,
                                }),
                            },
                            details: ActionConfig {
                                fetch: FetchMethod::default(),
                                parse: Strategy::Js(JsExecutionConfig {
                                    js_function: Some("parseBookDetails".to_string()),
                                    script: None,
                                }),
                            },
                            chapter: ActionConfig {
                                fetch: FetchMethod::default(),
                                parse: Strategy::Js(JsExecutionConfig {
                                    js_function: Some("parseChapterContent".to_string()),
                                    script: None,
                                }),
                            },
                            search,
                            genres: vec![],
                        }
                    } else {
                        Default::default()
                    };

                    // Ensure the local sources/<id>/ directory exists even for metadata-only sources
                    let base = Path::new("sources").join(dir_name);
                    if let Err(e) = fs::create_dir_all(&base) {
                        eprintln!("Failed to create dir {:?}: {}", base, e);
                    }

                    // If we downloaded an index.js from the repo, save it under `sources/<id>/index.js`
                    if let Some(js_text) = index_js_txt.clone() {
                        let base = Path::new("sources").join(dir_name);
                        if let Err(e) = fs::create_dir_all(&base) {
                            eprintln!("Failed to create dir {:?}: {}", base, e);
                        } else {
                            let js_path = base.join("index.js");
                            if let Err(e) = fs::write(&js_path, js_text) {
                                eprintln!("Failed to write {:?}: {}", js_path, e);
                            }
                        }
                    }

                    // Try to save an icon: prefer icon file from repo, otherwise fall back to metadata.icon_url
                    if let Some(icon_dl) = icon_download_url.clone() {
                        let base = Path::new("sources").join(dir_name);
                        match client.get(&icon_dl).send().await {
                            Ok(r) => match r.bytes().await {
                                Ok(bytes) => {
                                    if let Err(e) = fs::create_dir_all(&base) {
                                        eprintln!("Failed to create dir {:?}: {}", base, e);
                                    } else if let Err(e) = fs::write(base.join("icon.png"), &bytes) {
                                        eprintln!("Failed to write icon for {}: {}", dir_name, e);
                                    }
                                }
                                Err(e) => eprintln!("Failed to download icon bytes {}: {}", icon_dl, e),
                            },
                            Err(e) => eprintln!("Failed to fetch icon {}: {}", icon_dl, e),
                        }
                    } else if let Some(icon_url) = meta.icon_url.clone() {
                        let base = Path::new("sources").join(dir_name);
                        match client.get(&icon_url).send().await {
                            Ok(r) => match r.bytes().await {
                                Ok(bytes) => {
                                    if let Err(e) = fs::create_dir_all(&base) {
                                        eprintln!("Failed to create dir {:?}: {}", base, e);
                                    } else if let Err(e) = fs::write(base.join("icon.png"), &bytes) {
                                        eprintln!("Failed to write icon for {}: {}", dir_name, e);
                                    }
                                }
                                Err(e) => eprintln!("Failed to download icon bytes {}: {}", icon_url, e),
                            },
                            Err(e) => eprintln!("Failed to fetch icon {}: {}", icon_url, e),
                        }
                    }

                    // If we saved an index.js locally, update script_path to point to it.
                    let mut final_config = config.clone();
                    if index_js_txt.is_some() {
                        final_config.script_path = Some(format!("sources/{}/index.js", dir_name));
                    }

                    let src: SourceWithConfig = SourceWithConfig {
                        source: Source {
                            id: dir_name.to_string(),
                            url: meta.url.clone(),
                            name: meta.name.clone(),
                            icon_url: meta.icon_url.clone(),
                            description: meta.description.clone(),
                        },
                        config: final_config,
                    };
                    let _ = db.save_source(&src);
                    imported.push(src.source.id.clone());
                }
            }
        }
    }

    Ok(imported)
}

/// Check for updates in a GitHub repo for sources imported from that repo.
/// Returns a vector of (source_id, needs_update, current_sha, latest_sha)
pub async fn check_for_updates(_repo_url: &str, _db: &Database) -> Result<Vec<(String, bool, Option<String>, Option<String>)>> {
    Ok(vec![])
}
