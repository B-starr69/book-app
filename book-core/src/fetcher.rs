use crate::configurable_fetcher::ConfigurableFetcher;
use crate::configurable_parser::ConfigurableParser;
use crate::models::{
    DynamicMode, FetchMethod, HomeSection, NativeTarget, ParseMethod,
    ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult,
    SourceWithConfig,
};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};

const _NATIVE_FETCH_MSG: &str = "Native fetching is not implemented yet";
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
            FetchMethod::Rust { target } => match target {
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
            FetchMethod::Rust { target } => match target {
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
            FetchMethod::Rust { target } => match target {
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
            FetchMethod::Rust { target } => match target {
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
            FetchMethod::Rust { target } => match target {
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

    async fn fetch_native_url(&self, url: &str) -> Result<String, String> {
        if url.is_empty() {
            return Err("Empty URL".to_string());
        }
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {e}"))?;
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response text: {e}"))?;
        Ok(text)
    }

    pub async fn get_chapter_list(
        &self,
        source: &SourceWithConfig,
        book_id: &str,
        page: i32,
    ) -> Result<Vec<ParsedChapterInfo>, String> {
        let html = match &source.config.chapters_list.fetch {
            FetchMethod::Rust { .. } => {
                let base_url = Self::resolve_chapters_list_url(source, book_id)
                    .unwrap_or_else(|| Self::resolve_details_url(source, book_id));
                let url = if page > 1 {
                    format!("{}?page={}", base_url, page)
                } else {
                    base_url
                };
                self.fetch_native_url(&url).await?
            }
            FetchMethod::Js => {
                let config = source.config.clone();
                let fetcher = ConfigurableFetcher::new(source.source.id.clone(), config);
                fetcher
                    .fetch_chapters_list(book_id, page)
                    .await
                    .map_err(|e| e.to_string())?
            }
            FetchMethod::HeadlessBrowser => {
                return Err("Headless browser fetching is not supported".to_string());
            }
        };

        match &source.config.chapters_list.parse {
            ParseMethod::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            ParseMethod::Js => {
                let config = source.config.clone();
                let parser = ConfigurableParser::new(source.source.id.clone(), config);
                parser.parse_chapters_only(&html).map_err(|e| e.to_string())
            }
        }
    }

    pub async fn get_book_details(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Result<ParsedBookDetails, String> {
        let html = match &source.config.details.fetch {
            FetchMethod::Rust { .. } => {
                let url = Self::resolve_details_url(source, &book_id);
                self.fetch_native_url(&url).await?
            }
            FetchMethod::Js => {
                let fetcher = ConfigurableFetcher::new(source.source.id.clone(), source.config.clone());
                fetcher
                    .fetch_details(&book_id)
                    .await
                    .map_err(|e| e.to_string())?
            }
            FetchMethod::HeadlessBrowser => {
                return Err("Headless browser fetching is not supported".to_string());
            }
        };

        match &source.config.details.parse {
            ParseMethod::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            ParseMethod::Js => {
                let parser = ConfigurableParser::new(source.source.id.clone(), source.config.clone());
                parser
                    .parse_book_details(&html, book_id)
                    .map_err(|e| e.to_string())
            }
        }
    }

    pub async fn get_home(&self, source: &SourceWithConfig) -> Result<Vec<HomeSection>, String> {
        // 1. Fetching Phase
        //let config_clone = source.config.clone();
        let html = match &source.config.home.fetch {
            FetchMethod::Rust { .. } => {
                let url = Self::resolve_home_url(source);
                self.fetch_native_url(&url).await.map_err(|e| {
                    format!(
                        "Native fetch failed for source '{}': {e}",
                        source.source.name
                    )
                })?
            }
            FetchMethod::Js { .. } => {
                // 1. Define the variables you need to move into the background thread
                let config_clone = source.config.clone();
                let source_id = source.source.id.clone();
                let source_name = source.source.name.clone(); // Cloning the name is a good practice to avoid holding references across the .await

                // 2. Run the blocking JS fetch on a background thread
                let fetcher = ConfigurableFetcher::new(source_id, config_clone);
                fetcher
                    .fetch_home()
                    .await
                    // 3. Handle Tokio's JoinError (if the background thread panicked)
                    .map_err(|e| {
                        format!(
                            "JS fetch script execution panicked for '{}': {e}",
                            source_name
                        )
                    })?

                // ^^^ NO SEMICOLON! This allows the String to be returned.
            }
            FetchMethod::HeadlessBrowser => {
                return Err(format!(
                    "Unsupported fetch method 'HeadlessBrowser' encountered for source '{}'",
                    source.source.name
                ));
            }
        };
        // 2. Parsing Phase
        match &source.config.home.parse {
            ParseMethod::Rust => Err(format!(
                "Rust native parsing strategy is not implemented yet (triggered by '{}')",
                source.source.name
            )),
            ParseMethod::Js => {
                let parser = ConfigurableParser::new(source.source.id.clone(), source.config.clone());
                parser.parse_home(&html, &source.source.url).map_err(|e| {
                    format!("JS parsing failed for source '{}': {e}", source.source.name)
                })
            }
        }
    }
    pub async fn get_chapter(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        chapter_id: String,
    ) -> Result<ParsedChapter, String> {
        let html = match &source.config.chapter.fetch {
            FetchMethod::Rust { .. } => {
                let url = Self::resolve_chapter_url(source, &book_id, &chapter_id);
                self.fetch_native_url(&url).await?
            }
            FetchMethod::Js { .. } => {
                let fetcher = ConfigurableFetcher::new(source.source.id.clone(), source.config.clone());
                fetcher
                    .fetch_chapter_content(&book_id, &chapter_id)
                    .await
                    .map_err(|e| e.to_string())?
            }
            FetchMethod::HeadlessBrowser => {
                return Err("Headless browser fetching is not supported".to_string());
            }
        };

        match &source.config.chapter.parse {
            ParseMethod::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            ParseMethod::Js => {
                let parser = ConfigurableParser::new(source.source.id.clone(), source.config.clone());
                parser
                    .parse_chapter_content(&html)
                    .map_err(|e| e.to_string())
            }
        }
    }

    pub async fn search_books(
        &self,
        source: &SourceWithConfig,
        keyword: &str,
        genre: Option<&str>,
    ) -> Result<Vec<SearchResult>, String> {
        let search_config = source
            .config
            .search
            .as_ref()
            .ok_or_else(|| "Search capability is not configured for this source".to_string())?;

        let html = match &search_config.fetch {
            FetchMethod::Rust { .. } => {
                let url = Self::resolve_search_url(source, keyword, genre)
                    .ok_or_else(|| "Failed to resolve search URL".to_string())?;
                self.fetch_native_url(&url).await?
            }
            FetchMethod::Js { .. } => {
                let fetcher = ConfigurableFetcher::new(source.source.id.clone(), source.config.clone());
                fetcher
                    .fetch_search(keyword, genre)
                    .await
                    .map_err(|e| e.to_string())?
            }
            FetchMethod::HeadlessBrowser => {
                return Err("Headless browser fetching is not supported".to_string());
            }
        };

        match &search_config.parse {
            ParseMethod::Rust => Err(NATIVE_PARSE_MSG.to_string()),
            ParseMethod::Js => {
                let parser = ConfigurableParser::new(source.source.id.clone(), source.config.clone());
                parser
                    .parse_search_results(&html)
                    .map_err(|e| e.to_string())
            }
        }
    }

}
