use crate::database::Database;
use crate::fetcher::Fetcher;
use crate::models::*;

/// Get a book with all details and chapter metadata.
/// Checks the DB cache first; if not found, fetches from web, caches, and returns.
pub async fn get_book(db: &Database, source: &SourceWithConfig, book_id: &str) -> Option<Book> {
    // Check cache first
    if let Ok(Some(cached)) = db.get_book(book_id, &source.source.id).await {
        return Some(cached);
    }

    // Fetch from web
    let fetcher = Fetcher::new();
    let details = fetcher
        .get_book_details(source, book_id.to_string())
        .await
        .ok()?;
    let parsed_chapters = fetcher
        .get_chapter_list(source, book_id, details.chapters_count)
        .await
        .ok()?;
    let chapters = parsed_chapters
        .into_iter()
        .map(|p| p.into_chapter())
        .collect();
    let book = build_book(book_id, source.source.id.clone(), details, chapters);
    let _ = db.save_book(&book).await;
    Some(book)
}

/// Get chapter content.
/// Checks the disk HTML cache first; if not found, fetches from web, caches, and returns.
pub async fn get_chapter_content(
    _db: &Database,
    source: &SourceWithConfig,
    book_id: &str,
    chapter_id: &str,
) -> Option<String> {
    // Check cache first
    if let Ok(Some(content)) =
        crate::storage::load_webnovel_chapter_html(&source.source.id, book_id, chapter_id)
    {
        return Some(content);
    }

    // Fetch from web
    let fetcher = Fetcher::new();
    let parsed = fetcher
        .get_chapter(source, book_id.to_string(), chapter_id.to_string())
        .await
        .ok()?;
    let _ = crate::storage::save_webnovel_chapter_html(
        &source.source.id,
        book_id,
        chapter_id,
        &parsed.content,
    );

    Some(parsed.content)
}

/// Get discover/home page sections for a source.
pub async fn get_discover_page(source: &SourceWithConfig) -> Option<Vec<HomeSection>> {
    let fetcher = Fetcher::new();

    match fetcher.get_home(source).await {
        Ok(sections) => Some(sections),
        Err(err) => {
            // Log the actual error to stderr or your logging framework
            eprintln!("Failed to fetch discover page for {:?}: {:?}", source, err);
            None
        }
    }
}
/// Search books by keyword on a single source, optionally filtered by genre.
/// `genre` should be a `GenreInfo::value` from the source's `genres` list.
pub async fn search_books(
    source: &SourceWithConfig,
    keyword: &str,
    genre: Option<&str>,
) -> Option<Vec<SearchResult>> {
    let fetcher = Fetcher::new();
    fetcher.search_books(source, keyword, genre).await.ok()
}

/// Get all books that are saved/added in the user's library.
pub async fn get_library_books(db: &Database) -> Option<Vec<Book>> {
    db.get_library_books().await.ok()
}

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

fn build_book(
    book_id: &str,
    source_id: String,
    details: ParsedBookDetails,
    chapters: Vec<Chapter>,
) -> Book {
    let base = BaseBook {
        id: book_id.to_string(),
        source_id,
        title: details.title,
        author: details.author,
        cover_url: details.cover_url,
        rating: details.rating,
        status: details.status,
        genres: details.genres,
        summary: details.summary,
        in_library: false,
        last_synced: Some(chrono::Utc::now().timestamp()),
        last_read_timestamp: 0,
    };

    if let Some(format) = details.format_hint {
        Book::Novel(Novel {
            base,
            format,
            file_path: None,
            progress: 0.0,
        })
    } else {
        Book::WebNovel(WebNovel {
            base,
            chapters_count: details.chapters_count,
            chapters_path: String::new(),
            chapters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseMode;
    use crate::defaults::novelfire_source;

    #[tokio::test]
    async fn test_add() {
        let database = Database::new(DatabaseMode::Local {
            path: "test.db".to_string(),
        })
        .await
        .unwrap();
        let sources = database.get_sources().await.unwrap();
        let novelfire = novelfire_source();
        let result = get_discover_page(&novelfire).await.unwrap();
    }
}
