use crate::configurable_parser::ConfigurableParser;
use crate::models::DynamicMode::Single;
use crate::models::*;
use crate::models::{
    ActionEngine, DynamicMode, FetchMethod, FetchMethod::Native, HomeSection, NativeTarget,
    ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult, SourceConfig,
    SourceWithConfig,
};
use crate::native_parser::NativeParser;
use crate::Chapter;
use core::panic;
use std::ops::Deref;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use rquickjs::function::Func;
use scraper::selector::Parser;
use std::path::PathBuf;
use tokio::fs;

/// Shared interface used by `fetch_book_with_chapters`.
trait BookParser {
    fn parse_book_details(&self, html: &str, book_id: String) -> Result<ParsedBookDetails, String>;
    fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>, String>;
}

impl BookParser for NativeParser {
    fn parse_book_details(&self, html: &str, book_id: String) -> Result<ParsedBookDetails, String> {
        self.parse_book_details(html, book_id)
    }
    fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>, String> {
        self.parse_chapters_only(html)
    }
}

impl BookParser for ConfigurableParser {
    fn parse_book_details(&self, html: &str, book_id: String) -> Result<ParsedBookDetails, String> {
        self.parse_book_details(html, book_id)
            .map_err(|e| e.to_string())
    }
    fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>, String> {
        self.parse_chapters_only(html).map_err(|e| e.to_string())
    }
}

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
    // URL resolution helpers — read ActionConfig.fetch for all URL construction
    // -------------------------------------------------------------------------
    /// Helper to trim trailing slashes from the base URL cleanly.
    /// not used as of now!
    /* fn get_base_url(source: &SourceWithConfig) -> &str {
        source.source.url.trim_end_matches('/')
    } */

    /// Resolves the home URL from `home.fetch`. Only handles static or non-paginated dynamic targets.
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
                _ => panic!(
                    "Expected a static or single-page dynamic target for home URL resolution"
                ),
            },
            _ => panic!("still not emplemented Js, HeadlessBrowser"),
        }
    }

    /// Build the book-details URL from `details.fetch`. Only handles static or non-paginated dynamic targets.
    /// Template placeholders: `{book_id}`, `{base_url}`.
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
                _ => {
                    panic!("Expected a static or single-page dynamic target for Details resolution")
                }
            },
            _ => panic!("still not emplemented Js, HeadlessBrowser"),
        }
    }
    /// Build the separate chapters-list URL using `chapters_list.fetch`. Only handles static or non-paginated dynamic targets.
    /// Template placeholders: `{book_id}`, `{base_url}`.
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
                _ => panic!(
                    "Expected a static or single-page dynamic target for Chapter List resolution"
                ),
            },
            _ => panic!("still not emplemented Js, HeadlessBrowser"),
        }
    }
    /// Build the chapter URL from `chapter.fetch`. Only handles static or non-paginated dynamic targets.
    /// Template placeholders: `{book_id}`, `{chapter_id}`, `{base_url}`.
    fn resolve_chapter_url(source: &SourceWithConfig, book_id: &str, chapter_id: &str) -> String {
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
                _ => {
                    panic!("Expected a static or single-page dynamic target for Details resolution")
                }
            },
            _ => panic!("still not emplemented Js, HeadlessBrowser"),
        }
    }
    /// Build the search URL from `search.fetch`. Only handles static or dynamic native targets.
    /// Template placeholders: `{keyword}`, `{genre}`, `{base_url}`.
    fn resolve_search_url(
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

                    let url = url_pattern
                        .replace("{keyword}", &encoded_keyword)
                        .replace("{genre}", &encoded_genre)
                        .replace("{base_url}", base_url);

                    Some(url)
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
        book_id: &str,
        nb_chap: i32,
    ) -> Result<Vec<ParsedChapterInfo>, String> {
        match &source.config.chapters_list.fetch {
            FetchMethod::Native { target } => match target {
                // 1. Static URL target or Dynamic Single (Non-Paginated)
                NativeTarget::Static { .. }
                | NativeTarget::Dynamic {
                    mode: DynamicMode::Single,
                    ..
                } => {
                    let url = Fetcher::resolve_chapters_list_url(source, book_id)
                        .ok_or_else(|| "Failed to resolve chapters list URL".to_string())?;

                    let html = self
                        .client
                        .get(url)
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .text()
                        .await
                        .map_err(|e| e.to_string())?;

                    let parser = NativeParser::new(source.config.clone());
                    parser.parse_chapters_only(&html)
                }

                // 2. Dynamic Paginated Target
                NativeTarget::Dynamic {
                    mode: DynamicMode::Paginated { config },
                    url_pattern,
                } => {
                    let mut chapters: Vec<ParsedChapterInfo> = Vec::new();
                    let parser = NativeParser::new(source.config.clone());

                    // Calculate total pages needed based on target chapters vs items per page
                    let nb_iter = Fetcher::div_ceil(nb_chap, config.nb_per_page);
                    let end_page = config.start_page + nb_iter;

                    let base_url = source.source.url.trim_end_matches('/');

                    for i in config.start_page..end_page {
                        let url = url_pattern
                            .replace("{base_url}", base_url)
                            .replace("{book_id}", book_id)
                            .replace("{page_number}", &i.to_string());

                        let html = self
                            .client
                            .get(url)
                            .send()
                            .await
                            .map_err(|e| e.to_string())?
                            .text()
                            .await
                            .map_err(|e| e.to_string())?;

                        // Parse this page's chapters and append them to our collection
                        let mut page_chapters = parser.parse_chapters_only(&html)?;
                        chapters.append(&mut page_chapters);
                    }

                    Ok(chapters)
                }
            },
            _ => Err("Engine type or combination not supported yet".to_string()),
        }
    }

    pub async fn get_book_details(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Result<ParsedBookDetails, String> {
        let html = match &source.config.details.fetch {
            FetchMethod::Native { .. } => {
                let url = Fetcher::resolve_details_url(source, &book_id);
                let resp = self
                    .client
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| format!("Network request failed: {}", e))?;

                resp.text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?
            }
            FetchMethod::Js { js_function: _func } => {
                todo!("implement js");
            }
            FetchMethod::HeadlessBrowser => {
                panic!("headless not impelemted ")
            }
        };
        match source.config.details.parse {
            Strategy::Rust(_) => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_book_details(&html, book_id)
            }
            Strategy::Js(_) => todo!("todo js parse"),
        }
    }

    pub async fn get_home(&self, source: &SourceWithConfig) -> Result<Vec<HomeSection>, String> {
        // 1. Fetch Stage: Resolve the target and grab the raw HTML payload
        let html = match &source.config.home.fetch {
            FetchMethod::Native { .. } => {
                let url = Fetcher::resolve_home_url(source);

                let resp = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Network request failed: {}", e))?;

                resp.text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?
            }
            FetchMethod::Js { js_function: _ } => {
                return Err(
                    "JS-driven fetching for home sections is not implemented yet".to_string(),
                );
            }
            FetchMethod::HeadlessBrowser => {
                return Err(
                    "Headless browser fetching for home sections is not implemented yet"
                        .to_string(),
                );
            }
        };

        // 2. Parse Stage: Pass the html string into the designated engine strategy
        match &source.config.home.parse {
            Strategy::Rust(_) => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_home(&html)
            }
            Strategy::Js(_) => {
                // Placeholder for your JS sandbox runner
                Err("JS parsing for home sections is not implemented yet".to_string())
            }
        }
    }

    pub async fn get_chapter(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        chapter_id: String,
    ) -> Result<ParsedChapter, String> {
        // 1. Fetch Stage: Identify the strategy, resolve the target URL, and fetch raw content
        let html = match &source.config.chapter.fetch {
            FetchMethod::Native { .. } => {
                let url = Fetcher::resolve_chapter_url(source, &book_id, &chapter_id);

                let resp = self
                    .client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Network request failed: {}", e))?;

                resp.text()
                    .await
                    .map_err(|e| format!("Failed to read response body: {}", e))?
            }
            FetchMethod::Js { js_function: _ } => {
                return Err(
                    "JS-driven fetching for chapter content is not implemented yet".to_string(),
                );
            }
            FetchMethod::HeadlessBrowser => {
                return Err(
                    "Headless browser fetching for chapter content is not implemented yet"
                        .to_string(),
                );
            }
        };

        // 2. Parse Stage: Route the raw html string directly to your designated extraction strategy
        match &source.config.chapter.parse {
            Strategy::Rust(_) => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_chapter_content(&html)
            }
            Strategy::Js(_) => {
                // Placeholder for your JS sandbox runner
                Err("JS parsing for chapter content is not implemented yet".to_string())
            }
        }
    }
    pub async fn search_books(
        self,
        source: &SourceWithConfig,
        keyword: &str,
        genre: Option<&str>,
    ) -> Result<Vec<SearchResult>, String> {
        let search_config = source
            .config
            .search
            .as_ref()
            .ok_or_else(|| "Search capability is not configured for this source".to_string())?;

        // 1. Fetch Stage: Resolve target URLs and fire network requests
        let resp = match &search_config.fetch {
            FetchMethod::Native { .. } => {
                let url = Fetcher::resolve_search_url(source, keyword, genre)
                    .ok_or_else(|| "Failed to resolve search target URL".to_string())?;

                self.client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| format!("Search network request failed: {}", e))?
            }
            FetchMethod::Js { js_function: _ } => {
                return Err("JS-driven fetching for searching is not implemented yet".to_string());
            }
            FetchMethod::HeadlessBrowser => {
                return Err(
                    "Headless browser fetching for searching is not implemented yet".to_string(),
                );
            }
        };

        // 2. Parse Stage: Read strategy configurations and hand off payload streams
        match &search_config.parse {
            Strategy::Rust(selectors) => match selectors {
                SearchSelectors::Json {
                    json_results_path,
                    mapping,
                } => {
                    let parser = NativeParser::new(source.config.clone());
                    parser.parse_json_search_results(
                        resp,
                        mapping.clone(),
                        json_results_path,
                    )
                    .await
                    .ok_or_else(|| "Failed to parse JSON search results".to_string())
                }
                SearchSelectors::Html {
                    item_selector,
                    mapping,
                } => {
                    let parser = NativeParser::new(source.config.clone());

                    parser
                    .parse_html_search_results(
                        resp,
                        item_selector,
                        mapping,
                    )
                    .await
                    .ok_or_else(|| "Failed to parse HTML search results".to_string())},
            },
            Strategy::Js(_) => {
                let parser = self.js_parser(source).await.ok_or_else(|| {
                    "Failed to initialize JavaScript parsing environment".to_string()
                })?;

                let html_payload = resp
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read search response stream: {}", e))?;

                parser
                    .parse_search_results(&html_payload)
                    .map_err(|e| format!("JavaScript search parsing execution failed: {}", e))
            }
            }
        }



    async fn js_parser(&self, source: &SourceWithConfig) -> Option<ConfigurableParser> {
        let config = self.prepare_js_config(source).await?;
        Some(ConfigurableParser::new(config))
    }

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
