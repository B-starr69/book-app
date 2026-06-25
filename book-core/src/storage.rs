//! Local file import for packaged books (Epub, Mobi, Pdf).
//!
//! Books are copied into app storage and referenced via `Book::file_path`.
//! There is no remote `download_url` — import always means a local file.

use crate::database::Database;
use crate::models::{BaseBook, Book, BookFormat, Chapter, Novel};
use crate::platform;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const LOCAL_SOURCE_ID: &str = "local";

pub fn detect_format(path: &Path) -> Option<BookFormat> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "epub" => Some(BookFormat::Epub),
        "mobi" => Some(BookFormat::Mobi),
        "pdf" => Some(BookFormat::Pdf),
        _ => None,
    }
}

fn book_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book")
        .to_string()
}

fn destination_path(format: &BookFormat, id: &str) -> PathBuf {
    let ext = match format {
        BookFormat::Epub => "epub",
        BookFormat::Mobi => "mobi",
        BookFormat::Pdf => "pdf",
    };
    platform::get_books_dir().join(format!("{id}.{ext}"))
}

fn safe_segment(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(value.as_bytes())
}

pub fn webnovel_dir(source_id: &str, book_id: &str) -> PathBuf {
    platform::get_webnovels_dir()
        .join(safe_segment(source_id))
        .join(safe_segment(book_id))
}

pub fn webnovel_chapters_index_path(source_id: &str, book_id: &str) -> PathBuf {
    webnovel_dir(source_id, book_id).join("chapters.json")
}

pub fn webnovel_chapter_html_path(source_id: &str, book_id: &str, chapter_id: &str) -> PathBuf {
    webnovel_dir(source_id, book_id)
        .join("chapters")
        .join(format!("{}.html", safe_segment(chapter_id)))
}

pub fn save_webnovel_chapter_index(
    source_id: &str,
    book_id: &str,
    chapters: &[Chapter],
) -> Result<PathBuf> {
    let index_path = webnovel_chapters_index_path(source_id, book_id);
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent).context("Failed to create webnovel chapter index directory")?;
    }

    let chapters: Vec<Chapter> = chapters
        .iter()
        .cloned()
        .map(|mut chapter| {
            chapter.file_path = Some(
                webnovel_chapter_html_path(source_id, book_id, &chapter.id)
                    .to_string_lossy()
                    .into_owned(),
            );
            chapter
        })
        .collect();
    let json =
        serde_json::to_string_pretty(&chapters).context("Failed to serialize chapter index")?;
    fs::write(&index_path, json).context("Failed to write webnovel chapter index")?;
    Ok(index_path)
}

pub fn load_webnovel_chapter_index(source_id: &str, book_id: &str) -> Result<Vec<Chapter>> {
    let index_path = webnovel_chapters_index_path(source_id, book_id);
    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let json = fs::read_to_string(index_path).context("Failed to read webnovel chapter index")?;
    let mut chapters: Vec<Chapter> =
        serde_json::from_str(&json).context("Failed to parse webnovel chapter index")?;
    for chapter in &mut chapters {
        if chapter.file_path.is_none() {
            chapter.file_path = Some(
                webnovel_chapter_html_path(source_id, book_id, &chapter.id)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    Ok(chapters)
}

pub fn save_webnovel_chapter_html(
    source_id: &str,
    book_id: &str,
    chapter_id: &str,
    html: &str,
) -> Result<PathBuf> {
    let path = webnovel_chapter_html_path(source_id, book_id, chapter_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create webnovel chapter directory")?;
    }
    fs::write(&path, html).context("Failed to write webnovel chapter HTML")?;
    Ok(path)
}

pub fn load_webnovel_chapter_html(
    source_id: &str,
    book_id: &str,
    chapter_id: &str,
) -> Result<Option<String>> {
    let path = webnovel_chapter_html_path(source_id, book_id, chapter_id);
    if !path.exists() {
        return Ok(None);
    }

    fs::read_to_string(path)
        .map(Some)
        .context("Failed to read webnovel chapter HTML")
}

fn extract_epub_metadata(path: &Path) -> Result<(String, String)> {
    let doc = epub::doc::EpubDoc::new(path).map_err(|e| anyhow!("Failed to parse epub: {e}"))?;

    let title = doc
        .mdata("title")
        .map(|item| item.value.clone())
        .unwrap_or_else(|| book_id_from_path(path));

    let author = doc
        .mdata("creator")
        .map(|item| item.value.clone())
        .unwrap_or_default();

    Ok((title, author))
}

/// Copy a local file into app storage and register it in the database.
 pub async fn import_local_file(db: &Database, source_path: &Path) -> Result<Book> {
    if !source_path.is_file() {
        return Err(anyhow!("Path is not a file: {}", source_path.display()));
    }

    let format = detect_format(source_path)
        .ok_or_else(|| anyhow!("Unsupported file type: {}", source_path.display()))?;

    let id = Uuid::new_v4().to_string();
    let dest = destination_path(&format, &id);
    fs::copy(source_path, &dest).context("Failed to copy book into app storage")?;

    let (title, author) = match format {
        BookFormat::Epub => extract_epub_metadata(&dest)
            .unwrap_or_else(|_| (book_id_from_path(source_path), String::new())),
        _ => (book_id_from_path(source_path), String::new()),
    };
    let base: BaseBook = BaseBook {
        id,
        source_id: LOCAL_SOURCE_ID.to_string(),
        title,
        author,
        cover_url: String::new(),
        status: String::new(),
        summary: String::new(),
        rating: 0.0,
        genres: Vec::new(),
        last_read_timestamp: 0,
        in_library: true,
        last_synced: None,
    };
    let book = Novel {
        base,
        format,
        file_path: Some(dest.to_string_lossy().into_owned()),
        progress: 0.0,
    };

    db.save_book(&Book::Novel(book.clone())).await?;
    Ok(Book::Novel(book))
}
