use crate::fetcher::Fetcher;
use crate::models::{
*};
use crate::defaults::novelfire_source;
use crate::database::Database;
/// Get a book with all details and chapter metadata.
/// Checks the DB cache first; if not found, fetches from web, caches, and returns.
pub async fn get_book(
    db: &Database,
    source: &SourceWithConfig,
    book_id: &str,
) -> Option<Book> {
    // Check cache first
    if let Ok(Some(cached)) = db.get_book(book_id, &source.source.id) {
        return Some(cached);
    }

    // Fetch from web
    let fetcher = Fetcher::new();
    let details = fetcher.get_book_details(source, book_id.to_string()).await.unwrap();
    let parsed_chapters = fetcher.get_chapter_list(source, book_id, details.chapters_count).await.unwrap();
    let chapters = parsed_chapters.iter().map(|p: &ParsedChapterInfo| p.from()).collect();
    let book = build_book(book_id, source.source.id.clone(), details,chapters);
    let _ = db.save_book(&book);
    Some(book)
}

/// Get chapter content.
/// Checks the DB cache first; if not found, fetches from web, caches, and returns.
pub async fn get_chapter_content(
    db: &Database,
    source: &SourceWithConfig,
    book_id: &str,
    chapter_id: &str,
) -> Option<ChapterContent> {
    // Check cache first
    if let Ok(Some(content)) = db.get_cached_chapter_content(book_id, &source.source.id, chapter_id) {
        // Find the chapter title from the book if available
        let title = db.get_book(book_id, &source.source.id)
            .ok()
            .flatten()
            .and_then(|book| {
                book.chapters.iter()
                    .find(|ch| ch.id == chapter_id)
                    .map(|ch| ch.title.clone())
            })
            .unwrap_or_default();

        return Some(ChapterContent {
            book_id: book_id.to_string(),
            source_id: source.source.id.clone(),
            chapter_id: chapter_id.to_string(),
            title,
            content,
        });
    }

    // Fetch from web
    let fetcher = Fetcher::new();
    let parsed = fetcher.get_chapter(source, book_id.to_string(), chapter_id.to_string()).await.unwrap();
    let _ = db.cache_chapter_content(book_id, &source.source.id, chapter_id, &parsed.content);

    Some(ChapterContent {
        book_id: book_id.to_string(),
        source_id: source.source.id.clone(),
        chapter_id: chapter_id.to_string(),
        title: parsed.title,
        content: parsed.content,
    })
}

/// Get discover/home page sections for a source.
pub async fn get_discover_page(source: &SourceWithConfig) -> Option<Vec<HomeSection>> {
    let fetcher = Fetcher::new();
    return Some(fetcher.get_home(source).await.unwrap())

}

/// Search books by keyword on a single source, optionally filtered by genre.
/// `genre` should be a `GenreInfo::value` from the source's `genres` list.
pub async fn search_books(
    source: &SourceWithConfig,
    keyword: &str,
    genre: Option<&str>,
) -> Option<Vec<SearchResult>> {
    let fetcher = Fetcher::new();
    return Some(fetcher.search_books(source, keyword, genre).await.unwrap())
}

/// Search books across multiple sources in parallel, optionally filtered by genre.
/* pub async fn search_all_sources(
    sources: &[SourceWithConfig],
    keyword: &str,
    genre: Option<&str>,
) -> Vec<SearchResult> {
    use futures::future::join_all;

    let futures: Vec<_> = sources
        .iter()
        .filter(|s| s.config.search.is_some())
        .map(|source| {
            let source = source.clone();
            let keyword = keyword.to_string();
            let genre = genre.map(|g| g.to_string());
            async move {
                let fetcher = Fetcher::new();
                match fetcher.search_books(&source, &keyword, genre.as_deref()).await {
                    Ok(mut results) => {
                        for result in &mut results {
                            result.source_id = Some(source.source.id.clone());
                            result.source_name = Some(source.source.name.clone());
                        }
                        results
                    }
                    None => vec![],
                }
            }
        })
        .collect();

    let results = join_all(futures).await;
    results.into_iter().flatten().collect()
}
 */
/// Get all books that are saved/added in the user's library.
pub fn get_library_books(db: &Database) -> Option<Vec<Book>> {
    db.get_library_books().ok()
}

// -------------------------------------------------------------------------
// Internal helpers
// -------------------------------------------------------------------------

fn build_book(book_id: &str, source: String, details: ParsedBookDetails,chapters: Vec<Chapter>) -> Book {
    Book {
        id: book_id.to_string(),
        source_id: source,
        title: details.title,
        author: details.author,
        cover_url: details.cover_url,
        rating: details.rating,
        status: details.status,
        chapters_count: details.chapters_count,
        genres: details.genres,
        summary: details.summary,
        in_library: false,
        last_read_timestamp: 0,
        chapters: chapters,
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports the add function from the parent scope

    #[tokio::test] // Identifies this function as a test case
    async fn test_add() {
        let database = Database::new().unwrap();
        let sources = database.get_sources().unwrap();
        let novelfire = novelfire_source();
        let result = get_discover_page(&novelfire).await.unwrap();
        println!("{:?}",result);
        println!("{:?}",sources);
    }
}