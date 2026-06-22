use serde::{Deserialize, Serialize};

// =========================================================================
// 1. Core Source Metadata
// =========================================================================
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Source {
    pub id: String,
    pub url: String,
    pub cover_url_pattern: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceWithConfig {
    #[serde(flatten)]
    pub source: Source,
    pub config: SourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenreInfo {
    pub name: String,
    pub value: String,
}

// =========================================================================
// 2. Book Format
// =========================================================================
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BookFormat {
    #[default]
    WebNovel,
    Epub,
    Mobi,
    Pdf,
}

// =========================================================================
// 3. Core Domain Models
// =========================================================================
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseBook {
    pub id: String,
    pub source_id: String,
    pub format: BookFormat,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub status: String,
    pub summary: String,
    pub rating: f32,
    pub genres: Vec<String>,
    pub in_library: bool,
    pub last_synced: i64,          // FIX: Added to match database schema
    pub last_read_timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Novel {
    #[serde(flatten)]
    pub base: BaseBook,
    pub file_path: String,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebNovel {
    #[serde(flatten)]
    pub base: BaseBook,
    pub chapters_count: i32,
    pub chapters_path: String,
    pub chapters: Vec<Chapter>,
}

pub enum Book {
    Novel(Novel),
    WebNovel(WebNovel)
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub file_path: Option<String>,
    pub date: Option<i64>,
    pub progress: f32,
    pub last_read: i64,
}

// =========================================================================
// 4. Data Transfer Objects & Pipeline Configs
// =========================================================================


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HomeSection {
    pub title: String,
    pub layout: SectionLayout,
    pub books: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SectionLayout {
    Horizontal,
    #[default]
    Grid,
    Ranking,
}

pub type SearchResult = String;

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
    pub format_hint: Option<BookFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapterInfo {
    pub id: String,
    pub title: String,
    pub date: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedChapter {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FetchMethod {
    Native { #[serde(flatten)] target: NativeTarget },
    Js { js_function: Option<String> },
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
    Static { url: String },
    Dynamic { url_pattern: String, #[serde(flatten)] mode: DynamicMode },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum DynamicMode {
    Single,
    Paginated { #[serde(flatten)] config: PaginationConfig },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationConfig {
    pub page_parameter: String,
    pub start_page: i32,
    pub nb_per_page: i32,
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
pub enum Strategy {
    Rust,
    Js(JsExecutionConfig),
}

impl Default for Strategy {
    fn default() -> Self {
        Strategy::Rust
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsExecutionConfig {
    pub js_function: Option<String>,
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionConfig {
    #[serde(default)]
    pub fetch: FetchMethod,
    #[serde(default)]
    pub parse: Strategy,
}

impl ActionConfig {
    pub fn effective_engine(&self) -> ActionEngine {
        match &self.parse {
            Strategy::Rust => ActionEngine::Rust,
            Strategy::Js(_) => ActionEngine::Js,
        }
    }

    pub fn js_script(&self) -> Option<&str> {
        match &self.parse {
            Strategy::Js(config) => config.script.as_deref(),
            Strategy::Rust => None,
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
            Strategy::Rust => None,
        }
    }

    pub fn set_js_function(&mut self, func: Option<String>) {
        if let Strategy::Js(ref mut config) = self.parse {
            config.js_function = func;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    pub script_path: Option<String>,
    #[serde(default)]
    pub default_format: BookFormat,
    #[serde(default)]
    pub home: ActionConfig,
    #[serde(default)]
    pub details: ActionConfig,
    #[serde(default)]
    pub chapter: ActionConfig,
    #[serde(default)]
    pub chapters_list: ActionConfig,
    pub search: Option<ActionConfig>,
    #[serde(default)]
    pub genres: Vec<GenreInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Repository {
    pub id: String,
    pub url: String,
    pub display_name: String,
    pub last_synced_commit: Option<String>,
    pub last_checked_timestamp: i64,
}

impl ParsedChapterInfo {
    pub fn into_chapter(self) -> Chapter {
        Chapter {
            id: self.id,
            title: self.title,
            file_path: None,
            date: self.date,
            progress: 0.0,
            last_read: 0,
        }
    }
}

impl Book {
    /// Shared helper to easily get read-only access to common core metadata fields
    pub fn base(&self) -> &BaseBook {
        match self {
            Book::Novel(n) => &n.base,
            Book::WebNovel(wn) => &wn.base,
        }
    }

    /// Shared helper to get mutable access to common core metadata fields
    pub fn base_mut(&mut self) -> &mut BaseBook {
        match self {
            Book::Novel(n) => &mut n.base,
            Book::WebNovel(wn) => &mut wn.base,
        }
    }

    pub fn id(&self) -> &str { &self.base().id }
    pub fn source_id(&self) -> &str { &self.base().source_id }
    pub fn format(&self) -> BookFormat { self.base().format }
}