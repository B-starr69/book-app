use crate::database::Database;
use crate::fetcher::Fetcher;
use crate::models::*;

/// Get a book with all details and chapter metadata.
/// Checks the DB cache first; if not found, fetches from web, caches, and returns.
pub async fn get_book(
    db: &Database,
    source: &SourceWithConfig,
    book_id: &str,
    fetch_chapters: bool,
    force_refresh: bool,
) -> Result<Book, String> {
    let mut cached_book = None;
    if let Ok(Some(cached)) = db.get_book(book_id, &source.source.id).await {
        let now = chrono::Utc::now().timestamp();
        // Check if the cached book is stale (older than 4 days = 345,600 seconds)
        let is_stale = match cached.base().last_synced {
            Some(last_synced) => (now - last_synced) > 4 * 24 * 60 * 60,
            None => true,
        };

        // If we requested chapters, but the cache has 0 chapters, it's incomplete
        let is_incomplete = match &cached {
            Book::WebNovel(webnovel) => fetch_chapters && webnovel.chapters.is_empty() && webnovel.chapters_count > 0,
            _ => false,
        };

        if !force_refresh && !is_stale && !is_incomplete {
            let cover_url = cached.base().cover_url.clone();
            let source_id = cached.source_id().to_string();
            let book_id = cached.id().to_string();
            tokio::spawn(async move {
                let _ = crate::storage::download_cover_if_needed(&source_id, &book_id, &cover_url).await;
            });
            return Ok(cached);
        }
        cached_book = Some(cached);
    }

    // Fetch from web
    let fetcher = Fetcher::new();
    let details_res = fetcher
        .get_book_details(source, book_id.to_string())
        .await;

    let details = match details_res {
        Ok(d) => d,
        Err(e) => {
            if let Some(cached) = cached_book {
                eprintln!("Warning: Failed to fetch book details: {e}. Falling back to cached version.");
                return Ok(cached);
            }
            return Err(format!("Failed to fetch book details: {e}"));
        }
    };

    let chapters = if fetch_chapters {
        let parsed_chapters_res = fetcher
            .get_chapter_list(source, book_id, 1)
            .await;

        let parsed_chapters = match parsed_chapters_res {
            Ok(c) => c,
            Err(e) => {
                if let Some(cached) = cached_book {
                    eprintln!("Warning: Failed to fetch chapter list: {e}. Falling back to cached version.");
                    return Ok(cached);
                }
                return Err(format!("Failed to fetch chapter list: {e}"));
            }
        };

        parsed_chapters
            .into_iter()
            .map(|p| p.into_chapter())
            .collect()
    } else {
        if let Some(Book::WebNovel(ref cached_webnovel)) = cached_book {
            cached_webnovel.chapters.clone()
        } else {
            Vec::new()
        }
    };

    let mut book = build_book(book_id, source.source.id.clone(), details, chapters);
    
    // Preserve user states and progress if we had a cached version
    if let Some(ref cached) = cached_book {
        let base_mut = book.base_mut();
        base_mut.in_library = cached.base().in_library;
        base_mut.last_read_timestamp = cached.base().last_read_timestamp;
        
        match (&mut book, cached) {
            (Book::Novel(novel), Book::Novel(cached_novel)) => {
                novel.progress = cached_novel.progress;
                novel.file_path = cached_novel.file_path.clone();
            }
            (Book::WebNovel(webnovel), Book::WebNovel(cached_webnovel)) => {
                webnovel.chapters_path = cached_webnovel.chapters_path.clone();
                for chapter in &mut webnovel.chapters {
                    if let Some(cached_chap) = cached_webnovel.chapters.iter().find(|c| c.id == chapter.id) {
                        chapter.progress = cached_chap.progress;
                        chapter.last_read = cached_chap.last_read;
                        chapter.file_path = cached_chap.file_path.clone();
                    }
                }
                
                // Add any other cached chapters that were not in the fetched page 1
                for cached_chap in &cached_webnovel.chapters {
                    if !webnovel.chapters.iter().any(|c| c.id == cached_chap.id) {
                        webnovel.chapters.push(cached_chap.clone());
                    }
                }
            }
            _ => {}
        }
    }

    let cover_url = book.base().cover_url.clone();
    let source_id = book.source_id().to_string();
    let book_id = book.id().to_string();
    tokio::spawn(async move {
        let _ = crate::storage::download_cover_if_needed(&source_id, &book_id, &cover_url).await;
    });

    if let Err(e) = db.save_book(&book).await {
        eprintln!("Warning: Failed to save book to cache: {e}");
    }
    Ok(book)
}

/// Get chapter content.
/// Checks the disk HTML cache first; if not found, fetches from web, caches, and returns.
pub async fn get_chapter_content(
    _db: &Database,
    source: &SourceWithConfig,
    book_id: &str,
    chapter_id: &str,
    force_refresh: bool,
) -> Result<String, String> {
    // Check cache first
    if !force_refresh {
        if let Ok(Some(content)) =
            crate::storage::load_webnovel_chapter_html(&source.source.id, book_id, chapter_id)
        {
            if !content.trim().is_empty() {
                return Ok(content);
            }
        }
    }

    // Fetch from web
    let fetcher = Fetcher::new();
    let parsed = fetcher
        .get_chapter(source, book_id.to_string(), chapter_id.to_string())
        .await
        .map_err(|e| format!("Failed to fetch chapter: {e}"))?;

    if parsed.content.trim().is_empty() {
        return Err("Fetched chapter content is empty".to_string());
    }

    if let Err(e) = crate::storage::save_webnovel_chapter_html(
        &source.source.id,
        book_id,
        chapter_id,
        &parsed.content,
    ) {
        eprintln!("Warning: Failed to save chapter html: {e}");
    }

    Ok(parsed.content)
}

/// Check the database for cached books by ID, returning only those that exist and are NOT stale.
pub async fn get_fresh_cached_books(
    db: &Database,
    source_id: &str,
    ids: Vec<String>,
) -> Result<Vec<Book>, String> {
    let mut fresh_books = Vec::new();
    let now = chrono::Utc::now().timestamp();
    
    for id in ids {
        if let Ok(Some(cached)) = db.get_book(&id, source_id).await {
            let is_stale = match cached.base().last_synced {
                Some(last_synced) => (now - last_synced) > 4 * 24 * 60 * 60,
                None => true,
            };
            if !is_stale {
                fresh_books.push(cached);
            }
        }
    }
    
    Ok(fresh_books)
}

/// Get discover/home page sections for a source.
pub async fn get_discover_page(source: &SourceWithConfig) -> Result<Vec<HomeSection>, String> {
    let fetcher = Fetcher::new();
    fetcher.get_home(source).await
}

/// Search books by keyword on a single source, optionally filtered by genre.
/// `genre` should be a `GenreInfo::value` from the source's `genres` list.
pub async fn search_books(
    source: &SourceWithConfig,
    keyword: &str,
    genre: Option<&str>,
) -> Result<Vec<SearchResult>, String> {
    let fetcher = Fetcher::new();
    fetcher.search_books(source, keyword, genre).await
}

/// Fetch a specific page of chapters from the web, save them to the DB, and return them.
pub async fn sync_chapters_page(
    db: &Database,
    source: &SourceWithConfig,
    book_id: &str,
    page: i32,
) -> Result<Vec<Chapter>, String> {
    let fetcher = Fetcher::new();
    let parsed_chapters = fetcher
        .get_chapter_list(source, book_id, page)
        .await
        .map_err(|e| format!("Failed to fetch chapter list: {e}"))?;

    let chapters: Vec<Chapter> = parsed_chapters
        .into_iter()
        .map(|p| p.into_chapter())
        .collect();

    db.save_chapters(book_id, &source.source.id, &chapters)
        .await
        .map_err(|e| format!("Failed to save chapters to cache: {e}"))?;

    Ok(chapters)
}

/// Get all books that are saved/added in the user's library.
pub async fn get_library_books(db: &Database) -> Result<Vec<Book>, String> {
    db.get_library_books().await.map_err(|e| e.to_string())
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
        let _sources = database.get_sources().await.unwrap();
        let novelfire = novelfire_source();
        let _result = get_discover_page(&novelfire).await.unwrap();
    }
}
