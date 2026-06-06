use serde::{Deserialize, Serialize};

// =========================================================================
// 1. Core Domain Models (The Single Source of Truth)
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Book {
    pub id: String,
    pub source_id: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub status: String,      // e.g., "Ongoing", "Completed"
    pub summary: String,
    pub rating: f32,
    pub chapters_count: i32,
    pub genres: Vec<String>,
    pub in_library: bool,
    pub last_read_timestamp: i64,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub progress: f32,       // 0.0 to 1.0
    pub last_read: i64,      // Unix timestamp
}

// =========================================================================
// 2. Content Payloads (On-Demand / Heavy Cache Only)
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterContent {
    pub book_id: String,
    pub source_id: String,
    pub chapter_id: String,
    pub title: String,
    pub content: String,
}

/// Raw parsed chapter data from a source page — internal use only.
/// Converted to `ChapterContent` at the API layer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapter {
    pub title: String,
    pub content: String,
    pub date: Option<String>,
}

// =========================================================================
// 3. Data Transfer Objects
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HomeSection {
    pub title: String,
    pub layout: SectionLayout,
    pub books: Vec<Book>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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

/// Raw parsed book details from a source page — internal use only.
/// Converted to `Book` at the API layer.
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

/// Raw parsed chapter metadata from a source page — internal use only.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapterInfo {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
}

// =========================================================================
// 4. Extensible Pipeline Engine Configuration
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionEngine {
    #[default]
    Rust,
    Js,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FetchMethod {
    Native {
        strategy: NativeFetch,
        target: UrlTarget,
    },
    Js {
        js_function: Option<String>,
    },
    HeadlessBrowser,
}

impl Default for FetchMethod {
    fn default() -> Self {
        FetchMethod::Native {
            strategy: NativeFetch::default(),
            target: UrlTarget::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UrlTarget {
    Static {
        url: String,
    },
    Template {
        url_pattern: String,
    },
}

impl Default for UrlTarget {
    fn default() -> Self {
        UrlTarget::Static {
            url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum NativeFetch {
    #[default]
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
// 5. Declarative Structural Selectors
// =========================================================================

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
    pub chapter_list: String,
    pub chapter_id_pattern: String,
    pub chapter_date: Option<String>,
    pub chapter_date_attr: Option<String>,
    pub chapter_id_template: Option<String>,
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

// =========================================================================
// 6. Source Structure
// =========================================================================

/// A genre that a source supports for filtered searching.
/// `value` is whatever the source puts in the URL (e.g. "action", "1").
/// `name` is the human-readable label shown in the UI (e.g. "Action").
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenreInfo {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    pub script_path: Option<String>,
    /// URL pattern for the separate chapters-list endpoint (if any).
    /// Template placeholders: `{book_id}`, `{base_url}`.
    /// e.g. `"{base_url}/{book_id}/chapters"`.
    pub chapters_list_url: Option<String>,
    pub home: ActionConfig<HomeSelectors>,
    pub details: ActionConfig<DetailsSelectors>,
    pub chapter: ActionConfig<ChapterSelectors>,
    pub search: Option<ActionConfig<SearchSelectors>>,
    /// Genres this source supports for filtered searching.
    /// If the search `url_pattern` contains `{genre}`, this list tells the
    /// UI which options are available.
    #[serde(default)]
    pub genres: Vec<GenreInfo>,
}

/// A source is a website/service that provides books.
/// URL routing is handled by `SourceConfig` via `ActionConfig.fetch`.
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

fn default_href_attr() -> String { "href".to_string() }
fn default_src_attr() -> String { "src".to_string() }
