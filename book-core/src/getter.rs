use crate::Chapter;
use crate::configurable_parser::ConfigurableParser;
use crate::models::DynamicMode::Single;
use crate::models::{
    ActionEngine, FetchMethod, HomeSection, ParsedBookDetails, ParsedChapter,
    ParsedChapterInfo, SearchResult, SourceConfig, SourceWithConfig, NativeTarget,FetchMethod::Native,DynamicMode
};
use crate::native_parser::NativeParser;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use core::panic;
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

    // -------------------------------------------------------------------------
    // URL resolution helpers — read ActionConfig.fetch for all URL construction
    // -------------------------------------------------------------------------
    /// Helper to trim trailing slashes from the base URL cleanly.
    /// not used as of now!
    /* fn get_base_url(source: &SourceWithConfig) -> &str {
        source.source.url.trim_end_matches('/')
    } */

    /// Resolves the home URL from `home.fetch`. Only handles static or non-paginated dynamic targets.
    fn resolve_home_url(source: &SourceWithConfig) -> String {
        match &source.config.home.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => url.clone(),
                NativeTarget::Dynamic {
                    url_pattern,
                    mode: DynamicMode::Single,
                } => url_pattern.replace("{base_url}", &source.source.url).trim_end_matches('/').to_string(),
                _ => panic!(
                    "Expected a static or single-page dynamic target for home URL resolution"
                ),
            },
            _ => panic!("still not emplemented Js, HeadlessBrowser"),
        }
    }

    /// Build the book-details URL from `details.fetch`. Only handles static or non-paginated dynamic targets.
    /// Template placeholders: `{book_id}`, `{base_url}`.
    fn resolve_details_url(source: &SourceWithConfig, book_id: &str) -> String {
        match &source.config.details.fetch {
            FetchMethod::Native { target } => match target {
                NativeTarget::Static { url } if !url.is_empty() => url.clone(),
                NativeTarget::Dynamic {
                    url_pattern,
                    mode: DynamicMode::Single,
                } => url_pattern
                    .replace("{book_id}", book_id)
                    .replace("{base_url}", &source.source.url.trim_end_matches('/')),
                _ => panic!("Expected a static or single-page dynamic target for Details resolution")
            }
            _ => panic!("still not emplemented Js, HeadlessBrowser")

        }
    }
    /// Build the separate chapters-list URL using `chapters_list.fetch`. Only handles static or non-paginated dynamic targets.
    /// Template placeholders: `{book_id}`, `{base_url}`.
    fn resolve_chapters_list_url(source: &SourceWithConfig, book_id: &str) -> Option<String> {
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
                _ => panic!("Expected a static or single-page dynamic target for Chapter List resolution"),
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
                _ => panic!("Expected a static or single-page dynamic target for Details resolution"),
            },
            _ => panic!("still not emplemented Js, HeadlessBrowser"),
        }
    }
    // -------------------------------------------------------------------------
    // Public fetchers
    // -------------------------------------------------------------------------
    pub async fn get_chapter_list(
    &self,
    source: &SourceWithConfig,
    book_id: &str,
) -> Option<Vec<Chapter>> {
    match &source.config.chapters_list.fetch {
        FetchMethod::Native { target } => match target {
            // 1. Static URL target
            NativeTarget::Static { .. } | NativeTarget::Dynamic { ..}  => {
                let url = Fetcher::resolve_chapters_list_url(source, book_id)?;
                let html = self.client.get(url).send().await.unwrap().text().await.unwrap();
                let parser = NativeParser::new(source.config);
                let t = parser.parse_chapters_only(&html).unwrap();
                
            }



            // 3. Dynamic Paginated target (Left blank for your custom implementation)
            NativeTarget::Dynamic { mode: DynamicMode::Paginated { config }, .. } => {
                // [Your custom pagination logic goes here]
                None
            }

            _ => None,
        },
        // JS engine or HeadlessBrowser execution blocks
        _ => {
            // TODO: Handle script or browser engines if applicable
            None
        }
    }
}
    pub async fn get_book_details(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Option<ParsedBookDetails> {
        let url = Self::resolve_details_url(source, &book_id);
        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.details.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                self.fetch_book_with_chapters(&parser, source, book_id, &html)
                    .await
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                self.fetch_book_with_chapters(&parser, source, book_id, &html)
                    .await
            }
        }
    }

    pub async fn get_book_metadata(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Option<ParsedBookDetails> {
        let url = Self::resolve_details_url(source, &book_id);
        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.details.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                let mut details = parser.parse_book_details(&html, book_id).ok()?;
                details.chapters.clear();
                Some(details)
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                let mut details = parser.parse_book_details(&html, book_id).ok()?;
                details.chapters.clear();
                Some(details)
            }
        }
    }

    pub async fn get_home(&self, source: &SourceWithConfig) -> Option<Vec<HomeSection>> {
        let url = Self::resolve_home_url(source);
        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.home.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_home(&html, &source.source.url).ok()
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                parser.parse_home(&html, &source.source.url).ok()
            }
        }
    }

    pub async fn get_chapter(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        chapter_id: String,
    ) -> Option<ParsedChapter> {
        let url = Self::resolve_chapter_url(source, &book_id, &chapter_id);
        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.chapter.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_chapter_content(&html).ok()
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                parser.parse_chapter_content(&html).ok()
            }
        }
    }

    pub async fn search_books(
        &self,
        source: &SourceWithConfig,
        keyword: &str,
        genre: Option<&str>,
    ) -> Option<Vec<SearchResult>> {
        let search_config = source.config.search.as_ref()?;
        let encoded_keyword = urlencoding::encode(keyword);
        let encoded_genre = genre
            .map(|g| urlencoding::encode(g).into_owned())
            .unwrap_or_default();

        let url = match &search_config.fetch {
            crate::models::FetchMethod::Native {
                target: crate::models::UrlTarget::Template { url_pattern },
                ..
            } => url_pattern
                .replace("{keyword}", &encoded_keyword)
                .replace("{genre}", &encoded_genre),
            crate::models::FetchMethod::Native {
                target: crate::models::UrlTarget::Static { url },
                ..
            } => url.clone(),
            _ => return None,
        };

        let resp = self.client.get(&url).send().await.ok()?;

        match search_config.effective_engine() {
            ActionEngine::Rust => {
                let selectors = match &search_config.parse {
                    crate::models::Strategy::Rust(ref s) => s,
                    _ => return None,
                };
                match &selectors.format {
                    crate::models::SearchPayloadFormat::Json {
                        json_results_path,
                        mapping,
                    } => {
                        self.parse_json_search_results(
                            resp,
                            json_results_path,
                            mapping,
                            selectors.cover_base_url.as_deref(),
                        )
                        .await
                    }
                    crate::models::SearchPayloadFormat::Html {
                        item_selector,
                        mapping,
                    } => {
                        self.parse_html_search_results(
                            resp,
                            source,
                            item_selector,
                            mapping,
                            selectors.cover_base_url.as_deref(),
                        )
                        .await
                    }
                }
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                let payload = resp.text().await.ok()?;
                parser.parse_search_results(&payload).ok()
            }
        }
    }

    /// Fetch a book's details page, parse it, and if the chapters list is empty,
    /// attempt to fetch chapters from a separate endpoint.
    async fn fetch_book_with_chapters<P: BookParser>(
        &self,
        parser: &P,
        source: &SourceWithConfig,
        book_id: String,
        html: &str,
    ) -> Option<ParsedBookDetails> {
        let mut details = parser.parse_book_details(html, book_id.clone()).ok()?;
        match source.config.chapters_list.fetch {
            FetchMethod::Native {
                strategy: NativeFetch::Single,
                target: UrlTarget::Template { url_pattern },
            } => {
                let url = url_pattern.replace("{book_id}", &book_id);
                if let Ok(resp) = self.client.get(url).send().await {
                    if let Ok(html) = resp.text().await {
                        if let Ok(chapters) = parser.parse_chapters_only(&html) {
                            details.chapters = chapters
                        }
                    }
                }
            }
            FetchMethod::Native {
                strategy: NativeFetch::Paginated { config },
                target: UrlTarget::Template { url_pattern },
            } => {}
        }
        if let Some(chapters_url) = Self::resolve_chapters_list_url(source, &book_id) {
            if let Ok(resp) = self.client.get(&chapters_url).send().await {
                if let Ok(chapters_html) = resp.text().await {
                    if let Ok(chapters) = parser.parse_chapters_only(&chapters_html) {
                        details.chapters = chapters;
                    }
                }
            }
        }

        Some(details)
    }

    async fn parse_json_search_results(
        &self,
        resp: reqwest::Response,
        json_results_path: &str,
        mapping: &crate::models::JsonSearchMapping,
        cover_base_url: Option<&str>,
    ) -> Option<Vec<SearchResult>> {
        let json: serde_json::Value = resp.json().await.ok()?;

        let results_array = if json_results_path.is_empty() {
            json.as_array()?
        } else {
            let mut current = &json;
            for key in json_results_path.split('.') {
                current = current.get(key)?;
            }
            current.as_array()?
        };

        let results: Vec<SearchResult> = results_array
            .iter()
            .filter_map(|item| {
                let id = item.get(&mapping.id_key)?.as_str()?.to_string();
                let title = item.get(&mapping.title_key)?.as_str()?.to_string();

                let cover_url = item
                    .get(&mapping.cover_key)
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        if s.starts_with("http://") || s.starts_with("https://") {
                            s.to_string()
                        } else if let Some(base) = cover_base_url {
                            format!("{}{}", base, s)
                        } else {
                            s.to_string()
                        }
                    })
                    .unwrap_or_default();

                let chapters_count = if !mapping.chapters_count_key.is_empty() {
                    item.get(&mapping.chapters_count_key)
                        .and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()))
                        .map(|n| n as i32)
                } else {
                    None
                };

                let genres = if let Some(ref genres_key) = mapping.genres_key {
                    if let Some(val) = item.get(genres_key) {
                        if let Some(arr) = val.as_array() {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        } else if let Some(s) = val.as_str() {
                            vec![s.to_string()]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                Some(SearchResult {
                    id,
                    title,
                    cover_url,
                    chapters_count,
                    source_id: None,
                    source_name: None,
                    genres,
                })
            })
            .collect();

        Some(results)
    }

    async fn parse_html_search_results(
        &self,
        resp: reqwest::Response,
        source: &SourceWithConfig,
        item_selector: &str,
        mapping: &crate::models::HtmlSearchMapping,
        cover_base_url: Option<&str>,
    ) -> Option<Vec<SearchResult>> {
        use regex::Regex;
        use scraper::{Html, Selector};

        let html = resp.text().await.ok()?;
        let document = Html::parse_document(&html);

        let item_sel = Selector::parse(item_selector).ok()?;
        let link_sel = if !mapping.link_selector.is_empty() {
            Selector::parse(&mapping.link_selector).ok()
        } else {
            None
        };
        let id_regex = if !mapping.id_pattern.is_empty() {
            Regex::new(&mapping.id_pattern).ok()
        } else {
            None
        };
        let title_sel = Selector::parse(&mapping.title_selector).ok()?;
        let cover_sel = if !mapping.cover_selector.is_empty() {
            Selector::parse(&mapping.cover_selector).ok()
        } else {
            None
        };
        let genres_sel = mapping
            .genres_selector
            .as_ref()
            .and_then(|s| Selector::parse(s).ok());

        let results: Vec<SearchResult> = document
            .select(&item_sel)
            .filter_map(|item| {
                let id = if let Some(ref lsel) = link_sel {
                    let link = item.select(lsel).next()?;
                    let href = link.value().attr("href")?;
                    if let Some(ref regex) = id_regex {
                        regex.captures(href)?.get(1)?.as_str().to_string()
                    } else {
                        href.to_string()
                    }
                } else {
                    return None;
                };

                let title = item
                    .select(&title_sel)
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string();

                let cover_url = cover_sel
                    .as_ref()
                    .and_then(|sel| {
                        let img = item.select(sel).next()?;
                        let src = img
                            .value()
                            .attr("src")
                            .or_else(|| img.value().attr("data-src"))?;
                        if src.starts_with("http") {
                            Some(src.to_string())
                        } else if let Some(base) = cover_base_url {
                            Some(format!("{}{}", base, src))
                        } else {
                            Some(format!("{}{}", source.source.url, src))
                        }
                    })
                    .unwrap_or_default();

                let genres = if let Some(ref sel) = genres_sel {
                    item.select(sel)
                        .map(|el| el.text().collect::<String>().trim().to_string())
                        .collect()
                } else {
                    vec![]
                };

                Some(SearchResult {
                    id,
                    title,
                    cover_url,
                    chapters_count: None,
                    source_id: None,
                    source_name: None,
                    genres,
                })
            })
            .collect();

        Some(results)
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
