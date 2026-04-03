use crate::getter::Downloader;
use crate::models::{HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult, SourceWithConfig};
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

/// Search books by keyword
pub async fn search_books(
    source: &SourceWithConfig,
    keyword: &str,
) -> Option<Vec<SearchResult>> {
    let dw = Downloader::new();
    dw.search_books(source, keyword).await
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
