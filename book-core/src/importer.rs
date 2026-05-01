use crate::database::Database;
use crate::models::SourceWithConfig;
use anyhow::{anyhow, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use url::Url;
use std::fs;
use std::path::PathBuf;

// QuickJS integration
use rquickjs::{Runtime, Context, Function, Value};

/// Metadata file structure stored in metadata.json
#[derive(Debug, serde::Deserialize)]
struct Metadata {
    id: String,
    name: String,
    url: String,
    discover_url: Option<String>,
    books_url: Option<String>,
    version: Option<String>,
    author: Option<String>,
    description: Option<String>,
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
            // We expect directories for each source
            if item.get("type").and_then(|t| t.as_str()) == Some("dir") {
                if let Some(dir_name) = item.get("name").and_then(|n| n.as_str()) {
                    // list contents of this directory
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
                    let mut has_metadata = false;
                    let mut metadata_txt = String::new();
                    let mut index_js_txt = None;
                    let mut icon_bytes = None;

                    if let Some(files_arr) = files.as_array() {
                        for f in files_arr {
                            if let Some(fname) = f.get("name").and_then(|n| n.as_str()) {
                                if fname == "metadata.json" {
                                    if let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) {
                                        metadata_txt = client
                                            .get(download_url)
                                            .header(USER_AGENT, "book-app-importer")
                                            .send()
                                            .await?
                                            .text()
                                            .await?;
                                        has_metadata = true;
                                    }
                                } else if fname == "index.js" {
                                    if let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) {
                                        let txt = client
                                            .get(download_url)
                                            .header(USER_AGENT, "book-app-importer")
                                            .send()
                                            .await?
                                            .text()
                                            .await?;
                                        index_js_txt = Some(txt);
                                    }
                                } else if fname.to_lowercase().starts_with("icon") {
                                    if let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) {
                                        let bytes = client
                                            .get(download_url)
                                            .header(USER_AGENT, "book-app-importer")
                                            .send()
                                            .await?
                                            .bytes()
                                            .await?;
                                        icon_bytes = Some(bytes.to_vec());
                                    }
                                }
                            }
                        }
                    }

                    if !has_metadata {
                        eprintln!("No metadata.json in sources/{} - skipping", dir_name);
                        continue;
                    }

                    // Parse metadata
                    let meta: Metadata = match serde_json::from_str(&metadata_txt) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("Failed to parse metadata.json in {}: {}", dir_name, e);
                            continue;
                        }
                    };

                    // Prepare local directory
                    let mut local_dir = PathBuf::from("sources");
                    local_dir.push(&meta.id);
                    let _ = fs::create_dir_all(&local_dir);

                    // Save metadata.json locally
                    let meta_path = local_dir.join("metadata.json");
                    let _ = fs::write(&meta_path, metadata_txt.as_bytes());

                    // Save index.js if present
                    let has_index = if let Some(js) = index_js_txt {
                        let idx_path = local_dir.join("index.js");
                        let _ = fs::write(&idx_path, js.as_bytes());
                        true
                    } else {
                        false
                    };

                    // Save icon if present
                    if let Some(bytes) = icon_bytes {
                        let icon_path = local_dir.join("icon.png");
                        let _ = fs::write(&icon_path, &bytes);
                    }

                    // If metadata contains a full config, try to persist to DB
                    // metadata.json may include a `config` field compatible with SourceWithConfig
                    if let Ok(cfg_src) = serde_json::from_str::<SourceWithConfig>(&fs::read_to_string(&meta_path)?) {
                        let _ = db.save_source_with_config(&cfg_src);
                        // Record origin commit SHA for updates
                        let commits_api = format!(
                            "https://api.github.com/repos/{}/{}/commits?path=sources/{}&per_page=1",
                            owner, repo, dir_name
                        );
                        if let Ok(comm_resp) = client
                            .get(&commits_api)
                            .header(USER_AGENT, "book-app-importer")
                            .header(ACCEPT, "application/vnd.github.v3+json")
                            .send()
                            .await
                        {
                            if comm_resp.status().is_success() {
                                if let Ok(comm_json) = comm_resp.json::<serde_json::Value>().await {
                                    if let Some(arr) = comm_json.as_array() {
                                        if let Some(first) = arr.first() {
                                            if let Some(sha) = first.get("sha").and_then(|s| s.as_str()) {
                                                let _ = db.update_source_origin(&cfg_src.id, repo_url, sha);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        imported.push(cfg_src.id.clone());
                    } else {
                        // Create minimal SourceWithConfig from metadata
                        let src = SourceWithConfig {
                            id: meta.id.clone(),
                            url: meta.url.clone(),
                            name: meta.name.clone(),
                            discover_url: meta.discover_url.clone().unwrap_or_default(),
                            books_url: meta.books_url.clone().unwrap_or_default(),
                            icon_url: None,
                            description: meta.description.clone(),
                            config: Default::default(),
                        };
                        let _ = db.save_source_with_config(&src);

                        // Record origin commit SHA for updates (best-effort)
                        let commits_api = format!(
                            "https://api.github.com/repos/{}/{}/commits?path=sources/{}&per_page=1",
                            owner, repo, dir_name
                        );
                        if let Ok(comm_resp) = client
                            .get(&commits_api)
                            .header(USER_AGENT, "book-app-importer")
                            .header(ACCEPT, "application/vnd.github.v3+json")
                            .send()
                            .await
                        {
                            if comm_resp.status().is_success() {
                                if let Ok(comm_json) = comm_resp.json::<serde_json::Value>().await {
                                    if let Some(arr) = comm_json.as_array() {
                                        if let Some(first) = arr.first() {
                                            if let Some(sha) = first.get("sha").and_then(|s| s.as_str()) {
                                                let _ = db.update_source_origin(&src.id, repo_url, sha);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        imported.push(src.id.clone());
                    }

                    // Execute index.js in QuickJS sandbox if present
                    if has_index {
                        if let Ok(js_code) = fs::read_to_string(local_dir.join("index.js")) {
                            // Run in quickjs
                            match run_quickjs_script(&js_code) {
                                Ok(_) => {
                                    // success - continue
                                }
                                Err(e) => {
                                    eprintln!("Script execution failed for {}: {}", meta.id, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(imported)
}

/// Check for updates in a GitHub repo for sources imported from that repo.
/// Returns a vector of (source_id, needs_update, current_sha, latest_sha)
pub async fn check_for_updates(repo_url: &str, db: &Database) -> Result<Vec<(String, bool, Option<String>, Option<String>)>> {
    let url = Url::parse(repo_url).map_err(|e| anyhow!("invalid repo url: {}", e))?;
    let segments: Vec<_> = url.path_segments().ok_or_else(|| anyhow!("invalid repo url"))?.collect();
    if segments.len() < 2 {
        return Err(anyhow!("invalid repo url"));
    }
    let owner = segments[0];
    let repo = segments[1];

    let mut results = Vec::new();
    let client = reqwest::Client::new();

    let sources = db.get_sources_by_origin(repo_url)?;
    for (source_id, current_opt) in sources {
        let commits_api = format!(
            "https://api.github.com/repos/{}/{}/commits?path=sources/{}&per_page=1",
            owner, repo, source_id
        );
        let mut latest_sha: Option<String> = None;
        if let Ok(resp) = client
            .get(&commits_api)
            .header(USER_AGENT, "book-app-importer")
            .header(ACCEPT, "application/vnd.github.v3+json")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(arr) = json.as_array() {
                        if let Some(first) = arr.first() {
                            if let Some(sha) = first.get("sha").and_then(|s| s.as_str()) {
                                latest_sha = Some(sha.to_string());
                            }
                        }
                    }
                }
            }
        }

        let needs_update = match (current_opt.as_deref(), latest_sha.as_deref()) {
            (Some(curr), Some(lat)) => curr != lat,
            (None, Some(_)) => true,
            _ => false,
        };

        results.push((source_id, needs_update, current_opt, latest_sha));
    }

    Ok(results)
}


/// Very small QuickJS runner that exposes limited host functions
fn run_quickjs_script(js: &str) -> Result<(), anyhow::Error> {
    // Create runtime and context
    let rt = Runtime::new().map_err(|e| anyhow!("quickjs runtime error: {}", e))?;
    let ctx = Context::full(&rt).map_err(|e| anyhow!("quickjs context error: {}", e))?;

    ctx.with(|ctx| {
        // Simple execution of the script for prototype (no host API yet)
        ctx.eval::<(), _>(js)?;
        Ok(())
    })
}
