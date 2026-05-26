use std::path::PathBuf;

pub fn get_cover_cache_path(source_id: &str, book_id: &str) -> PathBuf {
    let covers_dir = PathBuf::from("covers");
    let base = format!("{}_{}", source_id, book_id);

    for ext in &["jpg", "jpeg", "png", "webp", "gif"] {
        let path = covers_dir.join(format!("{}.{}", base, ext));
        if path.exists() {
            return path;
        }
    }
    covers_dir.join(base)
}

pub fn get_cached_cover_path(source_id: &str, book_id: &str) -> Option<PathBuf> {
    let path = get_cover_cache_path(source_id, book_id);
    if path.exists() && path.extension().is_some() {
        Some(path)
    } else {
        None
    }
}

pub fn cache_cover_sync(client: &reqwest::blocking::Client, source_id: &str, book_id: &str, cover_url: &str) {
    if cover_url.is_empty() {
        return;
    }

    let existing = get_cover_cache_path(source_id, book_id);
    if existing.exists() && existing.extension().is_some() {
        return;
    }

    let _ = std::fs::create_dir_all("covers");

    if let Ok(response) = client.get(cover_url).send() {
        if !response.status().is_success() {
            return;
        }

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
        } else if cover_url.contains(".png") {
            "png"
        } else if cover_url.contains(".webp") {
            "webp"
        } else {
            "jpg"
        };

        if let Ok(bytes) = response.bytes() {
            if bytes.len() < 8 {
                return;
            }

            let is_image = bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                || bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47])
                || bytes.starts_with(b"RIFF")
                || bytes.starts_with(b"GIF");

            if !is_image {
                return;
            }

            let covers_dir = PathBuf::from("covers");
            let path = covers_dir.join(format!("{}_{}.{}", source_id, book_id, ext));
            let _ = std::fs::write(&path, &bytes);
        }
    }
}
