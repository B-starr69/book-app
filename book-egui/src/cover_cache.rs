use std::path::PathBuf;

/// Get the path for a cached cover image (checks multiple extensions)
pub fn get_cover_cache_path(source_id: &str, book_id: &str) -> PathBuf {
    let covers_dir = PathBuf::from("covers");
    let base = format!("{}_{}", source_id, book_id);

    // Check for existing files with any extension
    for ext in &["jpg", "jpeg", "png", "webp", "gif"] {
        let path = covers_dir.join(format!("{}.{}", base, ext));
        if path.exists() {
            return path;
        }
    }

    // Default to no extension (will be added during download)
    covers_dir.join(base)
}

/// Check if cover is cached and return the file:// URL if so
pub fn get_cached_cover_url(source_id: &str, book_id: &str) -> Option<String> {
    let path = get_cover_cache_path(source_id, book_id);
    if path.exists() && path.extension().is_some() {
        // Return absolute file path for egui
        std::fs::canonicalize(&path)
            .ok()
            .map(|p| format!("file://{}", p.display()))
    } else {
        None
    }
}

/// Download and cache a cover image (blocking, call from background thread)
pub fn cache_cover_sync(source_id: &str, book_id: &str, cover_url: &str) {
    if cover_url.is_empty() {
        return;
    }

    // Check if already cached
    let existing = get_cover_cache_path(source_id, book_id);
    if existing.exists() && existing.extension().is_some() {
        return; // Already cached
    }

    // Ensure covers directory exists
    let _ = std::fs::create_dir_all("covers");

    // Download the image with proper headers
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());

    match client.get(cover_url).send() {
        Ok(response) => {
            if !response.status().is_success() {
                println!(
                    "Failed to download cover: HTTP {} for URL: {}",
                    response.status(),
                    cover_url
                );
                return;
            }

            // Detect format from content-type or URL
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            let ext = if content_type.contains("png") {
                "png"
            } else if content_type.contains("webp") {
                "webp"
            } else if content_type.contains("gif") {
                "gif"
            } else if content_type.contains("jpeg") || content_type.contains("jpg") {
                "jpg"
            } else {
                // Guess from URL
                if cover_url.contains(".png") {
                    "png"
                } else if cover_url.contains(".webp") {
                    "webp"
                } else if cover_url.contains(".gif") {
                    "gif"
                } else {
                    "jpg"
                }
            };

            match response.bytes() {
                Ok(bytes) => {
                    // Verify it's actually an image (check magic bytes)
                    if bytes.len() < 8 {
                        println!("Cover too small, likely not an image");
                        return;
                    }

                    // Check magic bytes
                    let is_image = bytes.starts_with(&[0xFF, 0xD8, 0xFF]) // JPEG
                        || bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) // PNG
                        || bytes.starts_with(b"RIFF") // WebP
                        || bytes.starts_with(b"GIF"); // GIF

                    if !is_image {
                        println!("Downloaded content is not an image (possibly HTML/blocked)");
                        return;
                    }

                    let covers_dir = PathBuf::from("covers");
                    let path = covers_dir.join(format!("{}_{}.{}", source_id, book_id, ext));

                    if let Err(e) = std::fs::write(&path, &bytes) {
                        println!("Failed to write cover: {}", e);
                    } else {
                        println!("Cached cover: {} ({} bytes)", path.display(), bytes.len());
                    }
                }
                Err(e) => println!("Failed to read cover bytes: {}", e),
            }
        }
        Err(e) => println!("Failed to download cover: {}", e),
    }
}
