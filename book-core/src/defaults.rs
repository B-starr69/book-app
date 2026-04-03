//! Default source configurations
//!
//! This module provides built-in source configurations that can be
//! inserted into the database on first run or when users want to reset.
//!
//! Sources can also be loaded from JSON files in the `sources/` directory.

use crate::models::{
    ChapterSelectors, DetailsSelectors, HomeSelectors, LayoutMapping, SearchConfig,
    SearchResultMapping, SectionLayout, SourceConfig, SourceWithConfig,
};
use std::fs;
use std::path::Path;

/// Load sources from JSON files in the sources/ directory
/// Returns a vector of successfully loaded sources
pub fn load_sources_from_files() -> Vec<SourceWithConfig> {
    let sources_dir = Path::new("sources");
    let mut sources = Vec::new();

    if !sources_dir.exists() {
        // Create the directory if it doesn't exist
        let _ = fs::create_dir_all(sources_dir);
        return sources;
    }

    if let Ok(entries) = fs::read_dir(sources_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<SourceWithConfig>(&content) {
                            Ok(source) => {
                                println!("[SOURCES] Loaded source '{}' from {:?}", source.name, path);
                                sources.push(source);
                            }
                            Err(e) => {
                                eprintln!("[SOURCES] Failed to parse {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[SOURCES] Failed to read {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    sources
}

/// Get the default NovelFire source configuration
pub fn novelfire_source() -> SourceWithConfig {
    SourceWithConfig {
        id: "novelfire".to_string(),
        url: "https://novelfire.net".to_string(),
        name: "NovelFire".to_string(),
        discover_url: "https://novelfire.net/home".to_string(),
        books_url: "https://novelfire.net/book".to_string(),
        config: novelfire_config(),
    }
}

/// Get the NovelFire parser configuration
pub fn novelfire_config() -> SourceConfig {
    SourceConfig {
        home: HomeSelectors {
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
            title_attr: None,  // Use text content
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
        },
        details: DetailsSelectors {
            title: ".novel-title".to_string(),
            author: ".author span[itemprop='author']".to_string(),
            cover: ".fixed-img .cover img".to_string(),
            cover_attr: "src".to_string(),
            cover_attr_alt: Some("data-src".to_string()),
            rating: ".rating .nub".to_string(),
            status: ".header-stats .completed, .header-stats .ongoing".to_string(),
            chapters_count: ".header-stats span strong".to_string(),
            genres: ".categories ul li a".to_string(),
            summary: ".summary .content".to_string(),  // No p tag - direct content
            // Chapters are on separate page: /book/{id}/chapters
            chapter_list: "ul.chapter-list li a".to_string(),
            chapter_id_pattern: r"/book/[^/]+/([^/?#]+)".to_string(),
            chapter_date: Some("time.chapter-update".to_string()),
            chapter_date_attr: None,
            // NovelFire uses predictable chapter-{n} pattern
            chapter_id_template: Some("chapter-{n}".to_string()),
        },
        chapter: ChapterSelectors {
            title: ".chapter-title".to_string(),
            content: "#content".to_string(),
            date: None,
            date_attr: None,
        },
        search: Some(SearchConfig {
            url_pattern: "https://novelfire.net/ajax/searchLive?keyword={keyword}&type=title".to_string(),
            response_type: "json".to_string(),
            json_results_path: "data".to_string(),
            mapping: SearchResultMapping {
                id: "slug".to_string(),
                title: "title".to_string(),
                cover: "image".to_string(),
                chapters_count: "total_chapter".to_string(),
                item_selector: String::new(),
                link_selector: String::new(),
                id_pattern: String::new(),
            },
            cover_base_url: "https://novelfire.net/".to_string(),
        }),
    }
}

/// Get all default source configurations
pub fn all_default_sources() -> Vec<SourceWithConfig> {
    vec![
        novelfire_source(),
        // Add more default sources here as needed
    ]
}

/// Example of how to create a new source config for a different site
/// Users can use this as a template
pub fn example_source_template() -> SourceWithConfig {
    SourceWithConfig {
        id: "example-site".to_string(),
        url: "https://example.com".to_string(),
        name: "Example Site".to_string(),
        discover_url: "https://example.com/browse".to_string(),
        books_url: "https://example.com/novel".to_string(),
        config: SourceConfig {
            home: HomeSelectors {
                section: ".book-list-section".to_string(),
                header: ".section-title".to_string(),
                item: ".book-item".to_string(),
                link: "a.book-link".to_string(),
                book_id_pattern: r"/novel/([^/?#]+)".to_string(),
                href_attr: "href".to_string(),
                cover: "img".to_string(),
                cover_attr: "src".to_string(),
                cover_attr_alt: None,
                title: ".book-title".to_string(),
                title_attr: None,
                layout_mapping: vec![],
            },
            details: DetailsSelectors {
                title: "h1.book-title".to_string(),
                author: ".author-name".to_string(),
                cover: ".book-cover img".to_string(),
                cover_attr: "src".to_string(),
                cover_attr_alt: None,
                rating: ".rating-score".to_string(),
                status: ".book-status".to_string(),
                chapters_count: ".chapter-count".to_string(),
                genres: ".genre-tag".to_string(),
                summary: ".book-description".to_string(),
                chapter_list: ".chapter-list a".to_string(),
                chapter_id_pattern: r"/chapter/([^/?#]+)".to_string(),
                chapter_date: Some(".chapter-date".to_string()),
                chapter_date_attr: None,
                chapter_id_template: None,  // Set to Some("chapter-{n}") if chapters have predictable IDs
            },
            chapter: ChapterSelectors {
                title: "h1.chapter-title".to_string(),
                content: ".chapter-content".to_string(),
                date: None,
                date_attr: None,
            },
            search: None,  // No search configured for this example
        },
    }
}
