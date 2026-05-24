use crate::configurable_parser::ConfigurableParser;
use crate::models::{
    ActionEngine, HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult,
    SourceConfig, SourceWithConfig,
};
use crate::native_parser::NativeParser;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use std::path::PathBuf;
use tokio::fs;

pub struct Downloader {
    client: reqwest::Client,
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader {
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

    pub async fn get_book_from_web(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Option<ParsedBookDetails> {
        self.get_book_from_web_with_cache(source, book_id, None).await
    }

    pub async fn get_book_from_web_with_cache(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        _cached_data: Option<(i32, std::collections::HashMap<i32, String>)>,
    ) -> Option<ParsedBookDetails> {
        let url = format!("{}/{}", source.books_url.trim_end_matches('/'), book_id);
        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.details.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                self.finish_book_details_rust(&parser, source, book_id, &html)
                    .await
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                self.finish_book_details_js(&parser, source, book_id, &html)
                    .await
            }
        }
    }

    pub async fn get_book_metadata_only(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Option<ParsedBookDetails> {
        let url = format!("{}/{}", source.books_url.trim_end_matches('/'), book_id);
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

    pub async fn stream_chapters(
        &self,
        source: &SourceWithConfig,
        book_id: &str,
        _chapters_count: i32,
        chapters_tx: std::sync::mpsc::Sender<Vec<ParsedChapterInfo>>,
    ) {
        let chapters_url = format!(
            "{}/{}/chapters",
            source.books_url.trim_end_matches('/'),
            book_id
        );

        let resp = match self.client.get(&chapters_url).send().await {
            Ok(resp) => resp,
            Err(_) => return,
        };
        let chapters_html = match resp.text().await {
            Ok(html) => html,
            Err(_) => return,
        };

        match source.config.details.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                if let Ok(chapters) = parser.parse_chapters_only(&chapters_html) {
                    let _ = chapters_tx.send(chapters);
                }
            }
            ActionEngine::Js => {
                if let Some(parser) = self.js_parser(source).await {
                    if let Ok(chapters) = parser.parse_chapters_only(&chapters_html) {
                        let _ = chapters_tx.send(chapters);
                    }
                }
            }
        }
    }

    pub async fn load_home(&self, source: &SourceWithConfig) -> Option<Vec<HomeSection>> {
        let resp = self.client.get(&source.discover_url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.home.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_home(&html, &source.url).ok()
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                parser.parse_home(&html, &source.url).ok()
            }
        }
    }

    pub async fn load_home_streaming(
        &self,
        source: &SourceWithConfig,
        section_tx: std::sync::mpsc::Sender<HomeSection>,
    ) -> Option<usize> {
        let resp = self.client.get(&source.discover_url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match source.config.home.effective_engine() {
            ActionEngine::Rust => {
                let parser = NativeParser::new(source.config.clone());
                parser.parse_home_streaming(&html, &source.url, section_tx).ok()
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                parser.parse_home_streaming(&html, &source.url, section_tx).ok()
            }
        }
    }

    pub async fn get_chapter_from_web(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        chapter_id: String,
    ) -> Option<ParsedChapter> {
        let url = format!(
            "{}/{}/{}",
            source.books_url.trim_end_matches('/'),
            book_id,
            chapter_id
        );

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
    ) -> Option<Vec<SearchResult>> {
        let search_config = source.config.search.as_ref()?;
        let encoded_keyword = urlencoding::encode(keyword);
        let url = search_config.url_pattern.replace("{keyword}", &encoded_keyword);

        let resp = self.client.get(&url).send().await.ok()?;

        match search_config.effective_engine() {
            ActionEngine::Rust => {
                if search_config.response_type == "json" {
                    self.parse_json_search_results(resp, search_config).await
                } else {
                    self.parse_html_search_results(resp, source, search_config).await
                }
            }
            ActionEngine::Js => {
                let parser = self.js_parser(source).await?;
                let payload = resp.text().await.ok()?;
                parser.parse_search_results(&payload).ok()
            }
        }
    }

    async fn finish_book_details_rust(
        &self,
        parser: &NativeParser,
        source: &SourceWithConfig,
        book_id: String,
        html: &str,
    ) -> Option<ParsedBookDetails> {
        let mut details = parser.parse_book_details(html, book_id.clone()).ok()?;

        if details.chapters.is_empty() {
            let chapters_url = format!(
                "{}/{}/chapters",
                source.books_url.trim_end_matches('/'),
                book_id
            );

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

    async fn finish_book_details_js(
        &self,
        parser: &ConfigurableParser,
        source: &SourceWithConfig,
        book_id: String,
        html: &str,
    ) -> Option<ParsedBookDetails> {
        let mut details = parser.parse_book_details(html, book_id.clone()).ok()?;

        if details.chapters.is_empty() {
            let chapters_url = format!(
                "{}/{}/chapters",
                source.books_url.trim_end_matches('/'),
                book_id
            );

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
        search_config: &crate::models::SearchConfig,
    ) -> Option<Vec<SearchResult>> {
        let json: serde_json::Value = resp.json().await.ok()?;

        let results_array = if search_config.json_results_path.is_empty() {
            json.as_array()?
        } else {
            let mut current = &json;
            for key in search_config.json_results_path.split('.') {
                current = current.get(key)?;
            }
            current.as_array()?
        };

        let mapping = &search_config.mapping;
        let results: Vec<SearchResult> = results_array
            .iter()
            .filter_map(|item| {
                let id = item.get(&mapping.id)?.as_str()?.to_string();
                let title = item.get(&mapping.title)?.as_str()?.to_string();

                let cover_url = item
                    .get(&mapping.cover)
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        if s.starts_with("http://") || s.starts_with("https://") {
                            s.to_string()
                        } else if !search_config.cover_base_url.is_empty() {
                            format!("{}{}", search_config.cover_base_url, s)
                        } else {
                            s.to_string()
                        }
                    })
                    .unwrap_or_default();

                let chapters_count = if !mapping.chapters_count.is_empty() {
                    item.get(&mapping.chapters_count)
                        .and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()))
                        .map(|n| n as i32)
                } else {
                    None
                };

                Some(SearchResult {
                    id,
                    title,
                    cover_url,
                    chapters_count,
                    source_id: None,
                    source_name: None,
                })
            })
            .collect();

        Some(results)
    }

    async fn parse_html_search_results(
        &self,
        resp: reqwest::Response,
        source: &SourceWithConfig,
        search_config: &crate::models::SearchConfig,
    ) -> Option<Vec<SearchResult>> {
        use regex::Regex;
        use scraper::{Html, Selector};

        let html = resp.text().await.ok()?;
        let document = Html::parse_document(&html);
        let mapping = &search_config.mapping;

        let item_sel = Selector::parse(&mapping.item_selector).ok()?;
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
        let title_sel = Selector::parse(&mapping.title).ok()?;
        let cover_sel = if !mapping.cover.is_empty() {
            Selector::parse(&mapping.cover).ok()
        } else {
            None
        };

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
                        let src = img.value().attr("src").or_else(|| img.value().attr("data-src"))?;
                        if src.starts_with("http") {
                            Some(src.to_string())
                        } else if !search_config.cover_base_url.is_empty() {
                            Some(format!("{}{}", search_config.cover_base_url, src))
                        } else {
                            Some(format!("{}{}", source.url, src))
                        }
                    })
                    .unwrap_or_default();

                Some(SearchResult {
                    id,
                    title,
                    cover_url,
                    chapters_count: None,
                    source_id: None,
                    source_name: None,
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
            && config.home.script.is_none()
            || matches!(config.details.effective_engine(), ActionEngine::Js)
                && config.details.script.is_none()
            || matches!(config.chapter.effective_engine(), ActionEngine::Js)
                && config.chapter.script.is_none()
            || config
                .search
                .as_ref()
                .map(|search| matches!(search.effective_engine(), ActionEngine::Js) && search.script.is_none())
                .unwrap_or(false);

        let file_script = if needs_file {
            Some(self.load_js_script(source).await?)
        } else {
            None
        };

        if matches!(config.home.effective_engine(), ActionEngine::Js) && config.home.script.is_none() {
            config.home.script = file_script.clone();
        }
        if matches!(config.details.effective_engine(), ActionEngine::Js) && config.details.script.is_none() {
            config.details.script = file_script.clone();
        }
        if matches!(config.chapter.effective_engine(), ActionEngine::Js) && config.chapter.script.is_none() {
            config.chapter.script = file_script.clone();
        }
        if let Some(search) = config.search.as_mut() {
            if matches!(search.effective_engine(), ActionEngine::Js) && search.script.is_none() {
                search.script = file_script;
            }
        }

        Some(config)
    }

    async fn load_js_script(&self, source: &SourceWithConfig) -> Option<String> {
        let script_path = source
            .config
            .script_path
            .as_deref()
            .unwrap_or("index.js");

        let resolved = Self::resolve_script_path(source, script_path);
        fs::read_to_string(resolved).await.ok()
    }

    fn resolve_script_path(source: &SourceWithConfig, script_path: &str) -> PathBuf {
        let path = PathBuf::from(script_path);
        if path.is_absolute() {
            path
        } else if path.components().count() == 1 && script_path == "index.js" {
            PathBuf::from("sources").join(&source.id).join(path)
        } else {
            path
        }
    }
}
