use crate::database::Database;
use crate::models::{
    ActionEngine, ChapterSelectors, DetailsSelectors, HomeSelectors, SearchConfig, SourceConfig,
    SourceWithConfig,
};
use anyhow::{anyhow, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use url::Url;

/// Metadata file structure stored in metadata.json
#[derive(Debug, serde::Deserialize)]
struct RepoSource {
    name: String,
    url: String,
    #[serde(rename = "discoverUrl")]
    discover_url: String,
    #[serde(rename = "booksUrl")]
    books_url: String,
    #[serde(rename = "iconUrl")]
    icon_url: Option<String>,
    description: Option<String>,
    version: Option<String>,
    author: Option<String>,
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
                        let search = meta
                            .search
                            .and_then(|value| serde_json::from_value::<SearchConfig>(value).ok())
                            .map(|mut search| {
                                search.engine = ActionEngine::Js;
                                if search.js_function.is_none() {
                                    search.js_function = Some("parseSearch".to_string());
                                }
                                search.script = None;
                                search
                            });

                        SourceConfig {
                            version: 1,
                            script_path: Some("index.js".to_string()),
                            home: HomeSelectors {
                                engine: ActionEngine::Js,
                                js_function: Some("parseHome".to_string()),
                                script: None,
                                ..Default::default()
                            },
                            details: DetailsSelectors {
                                engine: ActionEngine::Js,
                                js_function: Some("parseBookDetails".to_string()),
                                script: None,
                                ..Default::default()
                            },
                            chapter: ChapterSelectors {
                                engine: ActionEngine::Js,
                                js_function: Some("parseChapterContent".to_string()),
                                script: None,
                                ..Default::default()
                            },
                            search,
                        }
                    } else {
                        Default::default()
                    };

                    let src = SourceWithConfig {
                        id: dir_name.to_string(),
                        url: meta.url.clone(),
                        name: meta.name.clone(),
                        discover_url: meta.discover_url.clone(),
                        books_url: meta.books_url.clone(),
                        icon_url: meta.icon_url.clone(),
                        description: meta.description.clone(),
                        config,
                    };
                    let _ = db.save_source_with_config(&src);
                    imported.push(src.id.clone());
                }
            }
        }
    }

    Ok(imported)
}

/// Check for updates in a GitHub repo for sources imported from that repo.
/// Returns a vector of (source_id, needs_update, current_sha, latest_sha)
pub async fn check_for_updates(repo_url: &str, db: &Database) -> Result<Vec<(String, bool, Option<String>, Option<String>)>> {
    Ok(vec![])
}
