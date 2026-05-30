use crate::getter::Downloader;
use crate::models::{HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult, SourceWithConfig};
use crate::{Book, Chapter, Database};
use std::sync::mpsc::Sender;

/// Get discover/home page sections for a source
pub async fn get_discover_page(source: SourceWithConfig) -> Option<Vec<HomeSection>> {
    let dw = Downloader::new();
    dw.load_home(&source).await
}

/// Get discover/home page sections with streaming - sends each section as it's parsed
/// Returns the total count of sections found
pub async fn get_discover_page_streaming(
    source: SourceWithConfig,
    section_tx: Sender<HomeSection>,
) -> Option<usize> {
    let dw = Downloader::new();
    dw.load_home_streaming(&source, section_tx).await
}

/// Get book details from web
pub async fn get_book_details(source: &SourceWithConfig, book_id: String) -> Option<ParsedBookDetails> {
    let dw = Downloader::new();
    dw.get_book_from_web(source, book_id).await
}

/// Get book details from cache when possible, otherwise fetch and cache
pub async fn get_book_details_cached(
    db: &Database,
    source: &SourceWithConfig,
    book_id: String,
) -> Option<Book> {
    if let Ok(Some(cached)) = db.get_full_book(&book_id, &source.id) {
        return Some(cached);
    }

    let details = get_book_details(source, book_id.clone()).await?;
    let book = build_book_from_details(book_id, source, details);
    let _ = db.save_full_book(&book);
    Some(book)
}

/// Get book metadata only (no chapters) - returns immediately
pub async fn get_book_metadata_only(
    source: &SourceWithConfig,
    book_id: String,
) -> Option<ParsedBookDetails> {
    let dw = Downloader::new();
    dw.get_book_metadata_only(source, book_id).await
}

/// Start streaming chapters in background - call after metadata is loaded
/// Spawns background task and returns immediately
pub fn start_chapter_streaming(
    source: SourceWithConfig,
    book_id: String,
    chapters_count: i32,
    chapters_tx: Sender<Vec<ParsedChapterInfo>>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let dw = Downloader::new();
            dw.stream_chapters(&source, &book_id, chapters_count, chapters_tx).await;
        });
    });
}

/// Get book details with cached data for incremental chapter fetching
/// cached_data: (cached_chapters_count, HashMap<chapter_num, title>)
pub async fn get_book_details_incremental(
    source: &SourceWithConfig,
    book_id: String,
    cached_data: Option<(i32, std::collections::HashMap<i32, String>)>,
) -> Option<ParsedBookDetails> {
    let dw = Downloader::new();
    dw.get_book_from_web_with_cache(source, book_id, cached_data).await
}

/// Get chapter content from web
pub async fn get_chapter_content(
    source: &SourceWithConfig,
    book_id: String,
    chapter_id: String,
) -> Option<ParsedChapter> {
    let dw = Downloader::new();
    dw.get_chapter_from_web(source, book_id, chapter_id).await
}

/// Get chapter content from cache when possible, otherwise fetch and cache
pub async fn get_chapter_content_cached(
    db: &Database,
    source: &SourceWithConfig,
    book_id: String,
    chapter_id: String,
) -> Option<String> {
    if let Ok(Some(content)) = db.get_cached_chapter_content(&book_id, &source.id, &chapter_id) {
        return Some(content);
    }

    let chapter = get_chapter_content(source, book_id.clone(), chapter_id.clone()).await?;
    let _ = db.cache_chapter_content(&book_id, &source.id, &chapter_id, &chapter.content);
    Some(chapter.content)
}

/// Get cover bytes from cache when possible, otherwise fetch and cache
pub async fn get_cover_bytes_cached(
    db: &Database,
    cover_url: &str,
    source_id: &str,
    book_id: &str,
) -> Option<Vec<u8>> {
    if cover_url.is_empty() {
        return None;
    }

    if let Ok(Some(bytes)) = db.get_cached_cover(book_id, source_id) {
        return Some(bytes);
    }

    println!(
        "Downloading cover: source={} book={} url={}",
        source_id, book_id, cover_url
    );

    let resp = reqwest::get(cover_url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 8 || !is_image_bytes(&bytes) {
        return None;
    }

    let data = bytes.to_vec();
    let _ = db.cache_cover(book_id, source_id, &data);
    Some(data)
}

/// Blocking version of cover fetch for non-Send database handles
pub fn get_cover_bytes_cached_blocking(
    db: &Database,
    cover_url: &str,
    source_id: &str,
    book_id: &str,
) -> Option<Vec<u8>> {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};

    if cover_url.is_empty() {
        return None;
    }

    if let Ok(Some(bytes)) = db.get_cached_cover(book_id, source_id) {
        return Some(bytes);
    }

    println!(
        "Downloading cover: source={} book={} url={}",
        source_id, book_id, cover_url
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
        ),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .ok()?;

    let resp = client.get(cover_url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().ok()?;
    if bytes.len() < 8 || !is_image_bytes(&bytes) {
        return None;
    }

    let data = bytes.to_vec();
    let _ = db.cache_cover(book_id, source_id, &data);
    Some(data)
}

/// Search books by keyword
pub async fn search_books(
    source: &SourceWithConfig,
    keyword: &str,
) -> Option<Vec<SearchResult>> {
    let dw = Downloader::new();
    dw.search_books(source, keyword).await
}

fn build_book_from_details(book_id: String, source: &SourceWithConfig, details: ParsedBookDetails) -> Book {
    Book {
        id: book_id,
        source_id: source.id.clone(),
        title: details.title,
        author: details.author,
        cover_url: details.cover_url,
        rating: details.rating,
        status: details.status,
        chapters_count: details.chapters_count,
        genres: details.genres,
        summary: details.summary,
        in_library: false,
        chapters: details
            .chapters
            .into_iter()
            .map(|c| Chapter {
                id: c.id,
                title: c.title,
                date: c.date,
                progress: 0.0,
                last_read: 0,
            })
            .collect(),
    }
}

fn is_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47])
        || bytes.starts_with(b"RIFF")
        || bytes.starts_with(b"GIF")
}

/// Search books across multiple sources in parallel
pub async fn search_all_sources(
    sources: &[SourceWithConfig],
    keyword: &str,
) -> Vec<SearchResult> {
    use futures::future::join_all;

    let futures: Vec<_> = sources
        .iter()
        .filter(|s| s.config.search.is_some())
        .map(|source| {
            let source = source.clone();
            let keyword = keyword.to_string();
            async move {
                let dw = Downloader::new();
                match dw.search_books(&source, &keyword).await {
                    Some(mut results) => {
                        // Add source_id and source_name to each result
                        for result in &mut results {
                            result.source_id = Some(source.id.clone());
                            result.source_name = Some(source.name.clone());
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

/// Search books across multiple sources with streaming - sends results as each source completes
/// This allows UI to show results incrementally instead of waiting for all sources
pub async fn search_all_sources_streaming(
    sources: &[SourceWithConfig],
    keyword: &str,
    results_tx: Sender<Vec<SearchResult>>,
) {
    for source in sources.iter().filter(|s| s.config.search.is_some()) {
        let dw = Downloader::new();
        if let Some(mut results) = dw.search_books(source, keyword).await {
            // Add source_id and source_name to each result
            for result in &mut results {
                result.source_id = Some(source.id.clone());
                result.source_name = Some(source.name.clone());
            }
            // Send results for this source immediately
            let _ = results_tx.send(results);
        }
    }
}
