use crate::database::Database;
use crate::models::{Repository, SourceWithConfig};
use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::header::{ACCEPT, USER_AGENT};
use std::fs;
use std::path::Path;
use url::Url;

/// Import sources from a GitHub repository's `sources/` directory.
/// Expects a structured folder ecosystem: sources/<id>/metadata.json, index.js
pub async fn import_from_github(
    repo_url: &str,
    base_dir: &Path, // FIX: Added base_dir to prevent hardcoded relative path issues
    db: &Database
) -> Result<Vec<String>> {
    // 1. Parse GitHub Repository URL
    let url = Url::parse(repo_url).map_err(|e| anyhow!("Invalid repository URL: {}", e))?;
    let segments: Vec<_> = url
        .path_segments()
        .ok_or_else(|| anyhow!("Invalid repository path strings"))?
        .filter(|s| !s.is_empty())
        .collect();

    if segments.len() < 2 {
        return Err(anyhow!("Repository URL must contain both an owner and a repo name"));
    }

    let owner = segments[0];
    let mut repo = segments[1].to_string();

    if repo.ends_with(".git") {
        repo = repo.trim_end_matches(".git").to_string();
    }

    let client = reqwest::Client::new();
    let repo_id = format!("{}_{}", owner, repo);

    let repository = Repository {
        id: repo_id.clone(),
        url: repo_url.to_string(),
        display_name: format!("{}/{}", owner, repo),
        last_synced_commit: None,
        last_checked_timestamp: Utc::now().timestamp(),
    };

    // FIX: Propagate database errors instead of silently ignoring them
    db.save_repository(&repository)?;

    let api_url = format!("https://api.github.com/repos/{}/{}/contents/sources", owner, repo);

    // 2. Fetch the directory index list
    let resp = client
        .get(&api_url)
        .header(USER_AGENT, "book-app-importer")
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("GitHub Directory API request returned status: {}", resp.status()));
    }

    let items: serde_json::Value = resp.json().await?;
    let mut imported_source_ids = Vec::new();

    let Some(directories_array) = items.as_array() else {
        return Ok(imported_source_ids);
    };

    // 3. Process every sub-directory found under `sources/`
    for item in directories_array {
        if item.get("type").and_then(|t| t.as_str()) != Some("dir") {
            continue;
        }
        let Some(dir_name) = item.get("name").and_then(|n| n.as_str()) else {
            continue;
        };

        let dir_api = format!(
            "https://api.github.com/repos/{}/{}/contents/sources/{}",
            owner, repo, dir_name
        );
        let dir_resp = client
            .get(&dir_api)
            .header(USER_AGENT, "book-app-importer")
            .header(ACCEPT, "application/vnd.github.v3+json")
            .send()
            .await?;

        if !dir_resp.status().is_success() {
            eprintln!("Skipping 'sources/{}': Content query failed with status {}", dir_name, dir_resp.status());
            continue;
        }

        let files: serde_json::Value = dir_resp.json().await?;
        let mut metadata_txt = String::new();
        let mut index_js_txt = None;
        let mut icon_download_url = None;

        // 4. Scan files inside the directory
        if let Some(files_arr) = files.as_array() {
            for f in files_arr {
                let Some(fname) = f.get("name").and_then(|n| n.as_str()) else { continue };
                let Some(download_url) = f.get("download_url").and_then(|d| d.as_str()) else { continue };

                match fname {
                    "metadata.json" => {
                        metadata_txt = client.get(download_url).send().await?.text().await?;
                    }
                    "index.js" => {
                        index_js_txt = Some(client.get(download_url).send().await?.text().await?);
                    }
                    n if n.starts_with("icon.") => {
                        icon_download_url = Some(download_url.to_string());
                    }
                    _ => {}
                }
            }
        }

        if metadata_txt.is_empty() {
            eprintln!("Skipping 'sources/{}': Required 'metadata.json' file is missing", dir_name);
            continue;
        }

        // 5. Strict Type Mapping
        let mut src: SourceWithConfig = match serde_json::from_str(&metadata_txt) {
            Ok(valid_model) => valid_model,
            Err(serde_err) => {
                eprintln!(
                    "Skipping 'sources/{}' due to critical strict metadata structural violations: {}",
                    dir_name, serde_err
                );
                continue;
            }
        };

        if src.source.id.is_empty() {
            src.source.id = dir_name.to_string();
        }

        // 6. Persistence: Create a matching folder block locally
        // FIX: Uses the passed base_dir instead of hardcoded "sources"
        let local_directory_base = base_dir.join("sources").join(&src.source.id);
        if let Err(e) = fs::create_dir_all(&local_directory_base) {
            eprintln!("Failed to generate local directory structure target {:?}: {}", local_directory_base, e);
            continue;
        }

        if let Some(js_payload) = index_js_txt {
            if let Err(e) = fs::write(local_directory_base.join("index.js"), js_payload) {
                eprintln!("Failed to write index.js configuration script to disk: {}", e);
            } else {
                src.config.script_path = Some(format!("sources/{}/index.js", src.source.id));
            }
        }

        if let Some(icon_target_url) = icon_download_url.or_else(|| src.source.icon_url.clone()) {
            if let Ok(icon_resp) = client.get(&icon_target_url).send().await {
                if let Ok(icon_bytes) = icon_resp.bytes().await {
                    let icon_path = local_directory_base.join("icon.png");
                    // FIX: Log error explicitly instead of silent `let _ =`
                    if let Err(e) = fs::write(&icon_path, icon_bytes) {
                        eprintln!("Failed to write icon to disk: {}", e);
                    } else {
                        src.source.icon_url = Some(format!("sources/{}/icon.png", src.source.id));
                    }
                }
            }
        }

        // 7. Store the verified data asset
        if let Err(db_err) = db.save_source_with_repo(&src, Some(&repo_id)) {
            eprintln!("Database transaction rejected source entry '{}': {:?}", src.source.id, db_err);
            continue;
        }

        imported_source_ids.push(src.source.id.clone());
    }

    // Update the repository's last checked timestamp upon completion
    if let Some(mut updated_repo) = db.get_repository(&repo_id)? {
        updated_repo.last_checked_timestamp = Utc::now().timestamp();
        // FIX: Propagate error
        db.save_repository(&updated_repo)?;
    }

    Ok(imported_source_ids)
}

/// Check for structural layout changes against your repository hashes
pub async fn check_for_updates(
    repo_url: &str,
    db: &Database,
) -> Result<Vec<(String, bool, Option<String>, Option<String>)>> {
    let url = Url::parse(repo_url).map_err(|e| anyhow!("Invalid repository URL: {}", e))?;
    let segments: Vec<_> = url
        .path_segments()
        .ok_or_else(|| anyhow!("Invalid repository path strings"))?
        .filter(|s| !s.is_empty())
        .collect();

    if segments.len() < 2 {
        return Err(anyhow!("Repository URL must contain both an owner and a repo name"));
    }

    let owner = segments[0];
    let mut repo = segments[1].to_string();
    if repo.ends_with(".git") {
        repo = repo.trim_end_matches(".git").to_string();
    }

    let repo_id = format!("{}_{}", owner, repo);
    let client = reqwest::Client::new();
    let commits_url = format!("https://api.github.com/repos/{}/{}/commits?per_page=1", owner, repo);

    let resp = client
        .get(&commits_url)
        .header(USER_AGENT, "book-app-importer")
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Failed to fetch commits: {}", resp.status()));
    }

    let commits: serde_json::Value = resp.json().await?;
    let latest_commit = commits.as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("sha"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let commit_message = commits.as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("commit"))
        .and_then(|c| c.get("message"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let mut results = Vec::new();

    if let Some(mut stored_repo) = db.get_repository(&repo_id)? {
        let has_update = match (&stored_repo.last_synced_commit, &latest_commit) {
            (Some(stored), Some(latest)) => stored != latest,
            (None, Some(_)) => true,
            _ => false,
        };

        stored_repo.last_checked_timestamp = Utc::now().timestamp();
        // FIX: Propagate error
        db.save_repository(&stored_repo)?;

        results.push((repo_id, has_update, latest_commit, commit_message));
    } else {
        results.push((repo_id, true, latest_commit, commit_message));
    }

    Ok(results)
}