use serde::{Deserialize, Serialize};

/// Search result for display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub cover_url: String,
    pub chapters_count: Option<i32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum SectionLayout {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[default]
    #[serde(rename = "grid")]
    Grid,
    #[serde(rename = "ranking")]
    Ranking,
}

/// A book preview with minimal data for discover page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookPreview {
    pub id: String,
    pub title: String,
    pub cover_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeSection {
    pub title: String,
    pub layout: SectionLayout,
    pub books: Vec<BookPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Book {
    pub id: String,
    pub source_id: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub rating: f32,
    pub status: String,
    pub chapters_count: i32,
    pub genres: Vec<String>,
    pub summary: String,
    pub in_library: bool,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapter {
    pub title: String,
    pub content: String,
    pub date: Option<String>,
}

/// Basic chapter info parsed from book details page (without content)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapterInfo {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedBookDetails {
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub rating: f32,
    pub status: String,
    pub chapters_count: i32,
    pub genres: Vec<String>,
    pub summary: String,
    pub chapters: Vec<ParsedChapterInfo>,
}

/// Represents a single chapter within a book, tracking reading status.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub progress: f32,
    pub last_read: i64,
}

/// Represents a content source (e.g., a specific website or API).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub name: String,
    pub discover_url: String,
    pub books_url: String,
}

/// Cache statistics for monitoring storage usage
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub chapter_count: i32,
    pub cover_count: i32,
    pub chapter_size_bytes: i64,
    pub cover_size_bytes: i64,
    pub total_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbBook {
    pub id: String,
    pub source_id: String,
    pub in_library: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbChapter {
    pub id: String,
    pub book_id: String,
    pub source_id: String,
    pub progress: f32,
    pub last_read: i64, // Unix timestamp
}

// ==================== Source Configuration ====================

/// Configuration for parsing a source's home/discover page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HomeSelectors {
    /// CSS selector for section containers
    pub section: String,
    /// CSS selector for section title/header
    pub header: String,
    /// CSS selector for book items within a section
    pub item: String,
    /// CSS selector for links within items
    pub link: String,
    /// Regex pattern to extract book ID from href (capture group 1)
    pub book_id_pattern: String,
    /// Optional: attribute to get href from (default: "href")
    #[serde(default = "default_href_attr")]
    pub href_attr: String,
    /// Optional: CSS selector for cover image within items
    #[serde(default)]
    pub cover: String,
    /// Optional: attribute to get cover URL from (default: "src")
    #[serde(default = "default_src_attr")]
    pub cover_attr: String,
    /// Optional: alternative cover attribute (e.g., "data-src")
    #[serde(default)]
    pub cover_attr_alt: Option<String>,
    /// Optional: CSS selector for title element within items
    #[serde(default)]
    pub title: String,
    /// Optional: attribute to get title from (default: text content, or use "title" for attribute)
    #[serde(default)]
    pub title_attr: Option<String>,
    /// Optional: mapping of section titles to layouts
    #[serde(default)]
    pub layout_mapping: Vec<LayoutMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutMapping {
    pub title_contains: String,
    pub layout: SectionLayout,
}

/// Configuration for parsing book details page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetailsSelectors {
    pub title: String,
    pub author: String,
    pub cover: String,
    /// Attribute to get cover URL from (default: "src")
    #[serde(default = "default_src_attr")]
    pub cover_attr: String,
    /// Alternative cover attribute (e.g., "data-src")
    #[serde(default)]
    pub cover_attr_alt: Option<String>,
    pub rating: String,
    pub status: String,
    pub chapters_count: String,
    pub genres: String,
    pub summary: String,
    /// Selector for chapter list items
    pub chapter_list: String,
    /// Regex pattern to extract chapter ID from href
    pub chapter_id_pattern: String,
    /// Optional: selector for chapter date
    #[serde(default)]
    pub chapter_date: Option<String>,
    /// Optional: attribute for chapter date
    #[serde(default)]
    pub chapter_date_attr: Option<String>,
    /// Optional: template for generating chapter IDs (e.g., "chapter-{n}")
    /// When set, chapters are generated using chapters_count instead of parsing HTML
    #[serde(default)]
    pub chapter_id_template: Option<String>,
}

/// Configuration for parsing chapter content page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterSelectors {
    pub title: String,
    pub content: String,
    /// Optional: selector for chapter date
    #[serde(default)]
    pub date: Option<String>,
    /// Optional: attribute for date (for meta tags)
    #[serde(default)]
    pub date_attr: Option<String>,
}

/// Configuration for search functionality
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchConfig {
    /// URL pattern with {keyword} placeholder, e.g., "https://example.com/search?q={keyword}"
    pub url_pattern: String,
    /// Response type: "json" or "html"
    #[serde(default = "default_response_type")]
    pub response_type: String,
    /// For JSON responses: JSON path to results array (e.g., "data" or "results.items")
    #[serde(default)]
    pub json_results_path: String,
    /// Field mappings for extracting book info from results
    #[serde(default)]
    pub mapping: SearchResultMapping,
    /// Base URL for building cover URLs (if images are relative)
    #[serde(default)]
    pub cover_base_url: String,
}

/// Mapping for extracting book data from search results
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResultMapping {
    /// JSON field or CSS selector for book ID/slug
    pub id: String,
    /// JSON field or CSS selector for book title
    pub title: String,
    /// JSON field or CSS selector for cover image URL
    #[serde(default)]
    pub cover: String,
    /// JSON field or CSS selector for chapter count (optional)
    #[serde(default)]
    pub chapters_count: String,
    /// For HTML: selector for search result items
    #[serde(default)]
    pub item_selector: String,
    /// For HTML: selector for link element
    #[serde(default)]
    pub link_selector: String,
    /// For HTML: regex pattern to extract ID from href
    #[serde(default)]
    pub id_pattern: String,
}

fn default_response_type() -> String {
    "json".to_string()
}

/// Complete source configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    pub home: HomeSelectors,
    pub details: DetailsSelectors,
    pub chapter: ChapterSelectors,
    /// Optional: search configuration
    #[serde(default)]
    pub search: Option<SearchConfig>,
}

fn default_href_attr() -> String {
    "href".to_string()
}

fn default_src_attr() -> String {
    "src".to_string()
}

/// Extended Source with embedded config
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceWithConfig {
    pub id: String,
    pub url: String,
    pub name: String,
    pub discover_url: String,
    pub books_url: String,
    pub config: SourceConfig,
}
