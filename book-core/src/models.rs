use serde::{Deserialize, Serialize};

// Helper functions for Serde defaults
fn default_href_attr() -> String { "href".to_string() }
fn default_src_attr() -> String { "src".to_string() }

// =========================================================================
// 1. Core Source Metadata
// =========================================================================

/// A source is a website/service that provides books.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub description: Option<String>,
}

/// A source bundled with its scraping/parsing configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceWithConfig {
    #[serde(flatten)]
    pub source: Source,
    pub config: SourceConfig,
}

/// Dynamic genre configuration that a source supports for filtered searching.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenreInfo {
    pub name: String, // Human-readable label (e.g., "Action")
    pub value: String, // Source-specific URL value (e.g., "action" or "1")
}

// =========================================================================
// 2. Core Domain Models (The Single Source of Truth)
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Book {
    pub id: String,
    pub source_id: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub status: String, // e.g., "Ongoing", "Completed"
    pub summary: String,
    pub rating: f32,
    pub chapters_count: i32,
    pub genres: Vec<String>,
    pub in_library: bool,
    pub last_read_timestamp: i64,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub progress: f32, // 0.0 to 1.0
    pub last_read: i64, // Unix timestamp
}

// =========================================================================
// 3. Data Transfer Objects & Scraper Payloads
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterContent {
    pub book_id: String,
    pub source_id: String,
    pub chapter_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HomeSection {
    pub title: String,
    pub layout: SectionLayout,
    pub books: Vec<Book>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SectionLayout {
    Horizontal,
    #[default]
    Grid,
    Ranking,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub cover_url: String,
    pub chapters_count: Option<i32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
}

// Internal-use only parsing structures
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapterInfo {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapter {
    pub title: String,
    pub content: String,
    pub date: Option<String>,
}

// =========================================================================
// 4. Extensible Pipeline Engine Configuration
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FetchMethod {
    Native {
        #[serde(flatten)]
        target: NativeTarget,
    },
    Js {
        js_function: Option<String>,
    },
    HeadlessBrowser,
}

impl Default for FetchMethod {
    fn default() -> Self {
        FetchMethod::Native {
            target: NativeTarget::Static { url: String::new() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NativeTarget {
    Static {
        url: String,
    },
    /// If it is not a static URL, it's considered a dynamic URL pattern (e.g., includes `{page}`, `{query}`)
    Dynamic {
        url_pattern: String,
        #[serde(flatten)]
        mode: DynamicMode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum DynamicMode {
    Single,
    Paginated {
        #[serde(flatten)]
        config: PaginationConfig,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationConfig {
    pub page_parameter: String,
    pub start_page: i32,
    pub nb_per_page: i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionEngine {
    #[default]
    Rust,
    Js,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "engine", rename_all = "snake_case")]
pub enum Strategy<T> {
    Rust(T),
    Js(JsExecutionConfig),
}

impl<T: Default> Default for Strategy<T> {
    fn default() -> Self {
        Strategy::Rust(T::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsExecutionConfig {
    pub js_function: Option<String>,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionConfig<T> {
    #[serde(default)]
    pub fetch: FetchMethod,
    pub parse: Strategy<T>,
}

impl<T> ActionConfig<T> {
    pub fn effective_engine(&self) -> ActionEngine {
        match &self.parse {
            Strategy::Rust(_) => ActionEngine::Rust,
            Strategy::Js(_) => ActionEngine::Js,
        }
    }

    pub fn js_script(&self) -> Option<&str> {
        match &self.parse {
            Strategy::Js(config) => config.script.as_deref(),
            _ => None,
        }
    }

    pub fn set_js_script(&mut self, script: Option<String>) {
        if let Strategy::Js(ref mut config) = self.parse {
            config.script = script;
        }
    }

    pub fn js_function(&self) -> Option<&str> {
        match &self.parse {
            Strategy::Js(config) => config.js_function.as_deref(),
            _ => None,
        }
    }

    pub fn set_js_function(&mut self, func: Option<String>) {
        if let Strategy::Js(ref mut config) = self.parse {
            config.js_function = func;
        }
    }
}

// =========================================================================
// 5. Declarative Structural HTML/JSON Selectors
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    pub script_path: Option<String>,
    pub home: ActionConfig<HomeSelectors>,
    pub details: ActionConfig<DetailsSelectors>,
    pub chapter: ActionConfig<ChapterSelectors>,
    pub chapters_list: ActionConfig<ChapterListSelector>,
    pub search: Option<ActionConfig<SearchSelectors>>,
    #[serde(default)]
    pub genres: Vec<GenreInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HomeSelectors {
    pub section: String,
    pub header: String,
    pub item: String,
    pub link: String,
    pub book_id_pattern: String,
    #[serde(default = "default_href_attr")]
    pub href_attr: String,
    pub cover: String,
    #[serde(default = "default_src_attr")]
    pub cover_attr: String,
    pub cover_attr_alt: Option<String>,
    pub title: String,
    pub title_attr: Option<String>,
    #[serde(default)]
    pub layout_mapping: Vec<LayoutMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutMapping {
    pub title_contains: String,
    pub layout: SectionLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetailsSelectors {
    pub title: String,
    pub author: String,
    pub cover: String,
    #[serde(default = "default_src_attr")]
    pub cover_attr: String,
    pub cover_attr_alt: Option<String>,
    pub rating: String,
    pub status: String,
    pub chapters_count: String,
    pub genres: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterListSelector {
    pub id: String,
    pub chapter_list: String,
    pub title: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterSelectors {
    pub title: String,
    pub content: String,
    pub date: Option<String>,
    pub date_attr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchSelectors {
    pub format: SearchPayloadFormat,
    pub cover_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "snake_case")]
pub enum SearchPayloadFormat {
    Json {
        json_results_path: String,
        mapping: JsonSearchMapping,
    },
    Html {
        item_selector: String,
        mapping: HtmlSearchMapping,
    },
}

impl Default for SearchPayloadFormat {
    fn default() -> Self {
        SearchPayloadFormat::Json {
            json_results_path: String::new(),
            mapping: JsonSearchMapping::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonSearchMapping {
    pub id_key: String,
    pub title_key: String,
    pub cover_key: String,
    pub chapters_count_key: String,
    pub genres_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HtmlSearchMapping {
    pub link_selector: String,
    pub id_pattern: String,
    pub title_selector: String,
    pub cover_selector: String,
    pub chapters_count_selector: String,
    pub genres_selector: Option<String>,
}