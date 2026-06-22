use crate::configurable_parser::ConfigurableParser;
use crate::models::{
    ActionEngine, DynamicMode, FetchMethod, HomeSection, NativeTarget, ParsedBookDetails,
    ParsedChapter, ParsedChapterInfo, SearchResult, SourceConfig, SourceWithConfig, Strategy,
};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use std::path::PathBuf;
use tokio::fs;

const NATIVE_FETCH_MSG: &str = "Native fetching is not implemented yet";
const NATIVE_PARSE_MSG: &str = "Native CSS selector parsing is not implemented yet";

/// Fetches web pages and parses them into domain objects using source configurations.
pub struct Fetcher {
    client: reqwest::Client,
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetcher {
    pub fn new() -> Self {
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

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    pub fn div_ceil(a: i32, b: i32) -> i32 {
        (a + b - 1) / b
    }



    // -------------------------------------------------------------------------
    // URL resolution helpers — kept for future native fetch reimplementation
    // -------------------------------------------------------------------------

    pub fn resolve_home_url(source: &SourceWithConfig) -> String {
        match &source.config.home.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => url.clone(),
                NativeTarget::Dynamic {
                    url_pattern,
                    mode: DynamicMode::Single,
                } => url_pattern
                    .replace("{base_url}", &source.source.url)
                    .trim_end_matches('/')
                    .to_string(),
                _ => String::new(),
            },
            _ => String::new(),
        }
    }

    pub fn resolve_details_url(source: &SourceWithConfig, book_id: &str) -> String {
        match &source.config.details.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => url.clone(),
                NativeTarget::Dynamic {
                    url_pattern,
                    mode: DynamicMode::Single,
                } => url_pattern
                    .replace("{book_id}", book_id)
                    .replace("{base_url}", &source.source.url.trim_end_matches('/')),
                _ => String::new(),
            },
            _ => String::new(),
        }
    }

    pub fn resolve_chapters_list_url(source: &SourceWithConfig, book_id: &str) -> Option<String> {
        match &source.config.chapters_list.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => Some(url.clone()),
                NativeTarget::Dynamic {
                    url_pattern,
                    mode: DynamicMode::Single,
                } => Some(
                    url_pattern
                        .replace("{book_id}", book_id)
                        .replace("{base_url}", &source.source.url.trim_end_matches('/')),
                ),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn resolve_chapter_url(
        source: &SourceWithConfig,
        book_id: &str,
        chapter_id: &str,
    ) -> String {
        match &source.config.chapter.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => url.clone(),
                NativeTarget::Dynamic {
                    url_pattern,
                    mode: DynamicMode::Single,
                } => url_pattern
                    .replace("{book_id}", book_id)
                    .replace("{chapter_id}", chapter_id)
                    .replace("{base_url}", &source.source.url.trim_end_matches('/')),
                _ => String::new(),
            },
            _ => String::new(),
        }
    }

    pub fn resolve_search_url(
        source: &SourceWithConfig,
        keyword: &str,
        genre: Option<&str>,
    ) -> Option<String> {
        let search_config = source.config.search.as_ref()?;

        match &search_config.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => Some(url.clone()),
                NativeTarget::Dynamic { url_pattern, .. } => {
                    let encoded_keyword = urlencoding::encode(keyword);
                    let encoded_genre = genre
                        .map(|g| urlencoding::encode(g).into_owned())
                        .unwrap_or_default();
                    let base_url = source.source.url.trim_end_matches('/');

                    Some(
                        url_pattern
                            .replace("{keyword}", &encoded_keyword)
                            .replace("{genre}", &encoded_genre)
                            .replace("{base_url}", base_url),
                    )
                }
                _ => None,
            },
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Public fetchers
    // -------------------------------------------------------------------------

    pub async fn get_chapter_list(
        &self,
        source: &SourceWithConfig,
        _book_id: &str,
        _nb_chap: i32,
    ) -> Result<Vec<ParsedChapterInfo>, String> {
        match &source.config.chapters_list.fetch {
            FetchMethod::Native { .. } => Err(NATIVE_FETCH_MSG.to_string()),
            FetchMethod::Js { .. } => {
                Err("JS fetching for chapter lists is not implemented yet".to_string())
            }
            FetchMethod::HeadlessBrowser => Err(
                "Headless browser fetching for chapter lists is not implemented yet".to_string(),
            ),
        }
    }

    pub async fn get_book_details(
        &self,
        source: &SourceWithConfig,
        _book_id: String,
    ) -> Result<ParsedBookDetails, String> {
        Self::ensure_fetch_supported(&source.config.details.fetch)?;
        match &source.config.details.parse {
            Strategy::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            Strategy::Js(_) => Err("JS parsing is not implemented yet".to_string()),
        }
    }

    pub async fn get_home(&self, source: &SourceWithConfig) -> Result<Vec<HomeSection>, String> {
        Self::ensure_fetch_supported(&source.config.home.fetch)?;
        match &source.config.home.parse {
            Strategy::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            Strategy::Js(_) => Err("JS parsing is not implemented yet".to_string()),
        }
    }

    pub async fn get_chapter(
        &self,
        source: &SourceWithConfig,
        _book_id: String,
        _chapter_id: String,
    ) -> Result<ParsedChapter, String> {
        Self::ensure_fetch_supported(&source.config.chapter.fetch)?;
        match &source.config.chapter.parse {
            Strategy::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            Strategy::Js(_) => Err("JS parsing is not implemented yet".to_string()),
        }
    }

    pub async fn search_books(
        self,
        source: &SourceWithConfig,
        _keyword: &str,
        _genre: Option<&str>,
    ) -> Result<Vec<SearchResult>, String> {
        let search_config = source
            .config
            .search
            .as_ref()
            .ok_or_else(|| "Search capability is not configured for this source".to_string())?;

        Self::ensure_fetch_supported(&search_config.fetch)?;
        match &search_config.parse {
            Strategy::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            Strategy::Js(_) => Err("JS parsing is not implemented yet".to_string()),
        }
    }

    fn ensure_fetch_supported(fetch: &FetchMethod) -> Result<(), String> {
        match fetch {
            FetchMethod::Native { .. } => Err(NATIVE_FETCH_MSG.to_string()),
            FetchMethod::Js { .. } => Err("JS fetching is not implemented yet".to_string()),
            FetchMethod::HeadlessBrowser => {
                Err("Headless browser fetching is not implemented yet".to_string())
            }
        }
    }

    #[allow(dead_code)]
    async fn js_parser(&self, source: &SourceWithConfig) -> Option<ConfigurableParser> {
        let config = self.prepare_js_config(source).await?;
        Some(ConfigurableParser::new(config))
    }

    #[allow(dead_code)]
    async fn prepare_js_config(&self, source: &SourceWithConfig) -> Option<SourceConfig> {
        let mut config = source.config.clone();
        let needs_file = matches!(config.home.effective_engine(), ActionEngine::Js)
            && config.home.js_script().is_none()
            || matches!(config.details.effective_engine(), ActionEngine::Js)
                && config.details.js_script().is_none()
            || matches!(config.chapter.effective_engine(), ActionEngine::Js)
                && config.chapter.js_script().is_none()
            || config
                .search
                .as_ref()
                .map(|search| {
                    matches!(search.effective_engine(), ActionEngine::Js)
                        && search.js_script().is_none()
                })
                .unwrap_or(false);

        let file_script = if needs_file {
            Some(self.load_js_script(source).await?)
        } else {
            None
        };

        if matches!(config.home.effective_engine(), ActionEngine::Js)
            && config.home.js_script().is_none()
        {
            config.home.set_js_script(file_script.clone());
        }
        if matches!(config.details.effective_engine(), ActionEngine::Js)
            && config.details.js_script().is_none()
        {
            config.details.set_js_script(file_script.clone());
        }
        if matches!(config.chapter.effective_engine(), ActionEngine::Js)
            && config.chapter.js_script().is_none()
        {
            config.chapter.set_js_script(file_script.clone());
        }
        if let Some(search) = config.search.as_mut() {
            if matches!(search.effective_engine(), ActionEngine::Js) && search.js_script().is_none()
            {
                search.set_js_script(file_script);
            }
        }

        Some(config)
    }

    #[allow(dead_code)]
    async fn load_js_script(&self, source: &SourceWithConfig) -> Option<String> {
        let script_path = source.config.script_path.as_deref().unwrap_or("index.js");

        let resolved = Self::resolve_script_path(source, script_path);
        fs::read_to_string(resolved).await.ok()
    }

    fn resolve_script_path(source: &SourceWithConfig, script_path: &str) -> PathBuf {
        let path = PathBuf::from(script_path);
        if path.is_absolute() {
            path
        } else if path.components().count() == 1 && script_path == "index.js" {
            PathBuf::from("sources").join(&source.source.id).join(path)
        } else {
            path
        }
    }
}
