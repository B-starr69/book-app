//! Default source configurations for hybrid (Rust/JS) scraping
//!
//! This module provides built-in source configurations that demonstrate
//! the hybrid approach: Rust for pure CSS selectors, JS when needed.

use crate::models::{
    ActionConfig, ChapterSelectors, DetailsSelectors, FetchMethod, HomeSelectors,
    JsonSearchMapping, LayoutMapping, NativeFetch, SearchPayloadFormat, SearchSelectors,
    SectionLayout, Source, SourceConfig, SourceWithConfig, Strategy, UrlTarget,
};

/// Get the default NovelFire source configuration (pure Rust)
///
/// NovelFire uses standard HTML with consistent CSS selectors,
/// so it's configured to use the native Rust parser for speed and efficiency.
pub fn novelfire_source() -> SourceWithConfig {
    SourceWithConfig {
        source: Source {
            id: "novelfire".to_string(),
            url: "https://novelfire.net".to_string(),
            name: "NovelFire".to_string(),
            icon_url: None,
            description: Some("Novel reading platform with CSS-based structure".to_string()),
        },
        config: novelfire_config(),
    }
}

/// Get the NovelFire parser configuration (Rust engine)
fn novelfire_config() -> SourceConfig {
    SourceConfig {
        script_path: None,
        chapters_list_url: None,
        home: ActionConfig {
            fetch: FetchMethod::default(),
            parse: Strategy::Rust(HomeSelectors {
                section: "section.container".to_string(),
                header: ".section-header h3".to_string(),
                item: ".novel-item".to_string(),
                link: "a".to_string(),
                book_id_pattern: r"/book/([^/?#]+)".to_string(),
                href_attr: "href".to_string(),
                cover: "img".to_string(),
                cover_attr: "src".to_string(),
                cover_attr_alt: Some("data-src".to_string()),
                title: "h4.novel-title".to_string(),
                title_attr: None,
                layout_mapping: vec![
                    LayoutMapping {
                        title_contains: "Recommend".to_string(),
                        layout: SectionLayout::Horizontal,
                    },
                    LayoutMapping {
                        title_contains: "Ranking".to_string(),
                        layout: SectionLayout::Ranking,
                    },
                ],
            }),
        },
        details: ActionConfig {
            fetch: FetchMethod::default(),
            parse: Strategy::Rust(DetailsSelectors {
                title: ".novel-title".to_string(),
                author: ".author span[itemprop='author']".to_string(),
                cover: ".fixed-img .cover img".to_string(),
                cover_attr: "src".to_string(),
                cover_attr_alt: Some("data-src".to_string()),
                rating: ".rating .nub".to_string(),
                status: ".header-stats .completed, .header-stats .ongoing".to_string(),
                chapters_count: ".header-stats span strong".to_string(),
                genres: ".categories ul li a".to_string(),
                summary: ".summary .content".to_string(),
                chapter_list: "ul.chapter-list li a".to_string(),
                chapter_id_pattern: r"/book/[^/]+/([^/?#]+)".to_string(),
                chapter_date: Some("time.chapter-update".to_string()),
                chapter_date_attr: None,
                chapter_id_template: Some("chapter-{n}".to_string()),
            }),
        },
        chapter: ActionConfig {
            fetch: FetchMethod::default(),
            parse: Strategy::Rust(ChapterSelectors {
                title: ".chapter-title".to_string(),
                content: "#content".to_string(),
                date: None,
                date_attr: None,
            }),
        },
        search: Some(ActionConfig {
            fetch: FetchMethod::Native {
                strategy: NativeFetch::Single,
                target: UrlTarget::Template {
                    url_pattern: "https://novelfire.net/ajax/searchLive?keyword={keyword}&type=title".to_string(),
                },
            },
            parse: Strategy::Rust(SearchSelectors {
                format: SearchPayloadFormat::Json {
                    json_results_path: "data".to_string(),
                    mapping: JsonSearchMapping {
                        id_key: "slug".to_string(),
                        title_key: "title".to_string(),
                        cover_key: "image".to_string(),
                        chapters_count_key: "total_chapter".to_string(),
                        genres_key: None,
                    },
                },
                cover_base_url: Some("https://novelfire.net/".to_string()),
            }),
        }),
        genres: vec![],
    }
}

/// Example hybrid source: uses Rust for home/search, but JS for complex chapter parsing
///
/// This demonstrates a source that needs JS to handle obfuscated chapter content
/// while using fast native parsing for discovery and search.
/* pub fn hybrid_example_source() -> SourceWithConfig {
    SourceWithConfig {
        id: "hybrid-example".to_string(),
        url: "https://example-hybrid.com".to_string(),
        name: "Hybrid Example".to_string(),
        discover_url: "https://example-hybrid.com/browse".to_string(),
        books_url: "https://example-hybrid.com/novel".to_string(),
        icon_url: None,
        description: Some("Example showing mixed Rust and JS parsing per action".to_string()),
        config: SourceConfig {
            version: 1,
            script_path: Some("index.js".to_string()),
            // Home discovery uses fast native selectors
            home: HomeSelectors {
                engine: ActionEngine::Rust,
                js_function: None,
                script: None,
                section: ".section".to_string(),
                header: ".section-title".to_string(),
                item: ".book-card".to_string(),
                link: "a.book-link".to_string(),
                book_id_pattern: r"/novel/(\d+)".to_string(),
                href_attr: "href".to_string(),
                cover: "img.cover".to_string(),
                cover_attr: "src".to_string(),
                cover_attr_alt: Some("data-src".to_string()),
                title: "h3.title".to_string(),
                title_attr: None,
                layout_mapping: vec![],
            },
            // Details use native parsing for metadata
            details: DetailsSelectors {
                engine: ActionEngine::Rust,
                js_function: None,
                script: None,
                title: ".book-info h1".to_string(),
                author: ".author-name".to_string(),
                cover: ".book-cover img".to_string(),
                cover_attr: "src".to_string(),
                cover_attr_alt: None,
                rating: ".rating-value".to_string(),
                status: ".status-badge".to_string(),
                chapters_count: ".chapter-count".to_string(),
                genres: ".genre-tag".to_string(),
                summary: ".synopsis".to_string(),
                chapter_list: ".chapter-list li a".to_string(),
                chapter_id_pattern: r"/chapter/(\d+)".to_string(),
                chapter_date: None,
                chapter_date_attr: None,
                chapter_id_template: None,
            },
            // Chapter content uses custom JS decryption
            chapter: ChapterSelectors {
                engine: ActionEngine::Js,
                js_function: Some("decryptAndParseChapter".to_string()),
                script: None, // Loaded from index.js at runtime
                title: ".chapter-title".to_string(),
                content: ".chapter-content".to_string(),
                date: None,
                date_attr: None,
            },
            // Search uses fast native JSON parsing
            search: Some(SearchConfig {
                engine: ActionEngine::Rust,
                js_function: None,
                script: None,
                url_pattern: "https://example-hybrid.com/api/search?q={keyword}".to_string(),
                response_type: "json".to_string(),
                json_results_path: "results".to_string(),
                mapping: SearchResultMapping {
                    id: "id".to_string(),
                    title: "title".to_string(),
                    cover: "thumbnail".to_string(),
                    chapters_count: "chapters".to_string(),
                    item_selector: String::new(),
                    link_selector: String::new(),
                    id_pattern: String::new(),
                },
                cover_base_url: String::new(),
            }),
        },
    }
}
 */
/// Get all default source configurations
pub fn all_default_sources() -> Vec<SourceWithConfig> {
    vec![
        novelfire_source(),
        // hybrid_example_source(), // Uncomment to test hybrid config
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActionEngine;

    #[test]
    fn novelfire_is_pure_rust() {
        let source = novelfire_source();
        assert_eq!(source.source.id, "novelfire");
        assert!(matches!(source.config.home.effective_engine(), ActionEngine::Rust));
        assert!(matches!(source.config.details.effective_engine(), ActionEngine::Rust));
        assert!(matches!(source.config.chapter.effective_engine(), ActionEngine::Rust));
    }
}
