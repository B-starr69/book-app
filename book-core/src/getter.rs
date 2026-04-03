use crate::configurable_parser::ConfigurableParser;
use crate::models::{HomeSection, ParsedBookDetails, ParsedChapter, SearchResult, SourceWithConfig};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};

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
        headers.insert(USER_AGENT, HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        ));
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

    /// Get book details with optional cached data for incremental updates
    /// If cached_chapters is provided, only fetches new chapters since last sync
    pub async fn get_book_from_web_with_cache(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        cached_data: Option<(i32, std::collections::HashMap<i32, String>)>, // (cached_count, cached_titles)
    ) -> Option<ParsedBookDetails> {
        let parser = ConfigurableParser::new(source.config.clone());
        let url = format!("{}/{}", source.books_url.trim_end_matches('/'), book_id);

        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match parser.parse_book_details(&html, book_id.clone()) {
            Ok(mut details) => {
                // If using chapter template, fetch actual titles from paginated pages
                if source.config.details.chapter_id_template.is_some() && details.chapters_count > 0 {
                    let chapters_with_titles = self.fetch_chapter_titles_incremental(
                        source,
                        &book_id,
                        &parser,
                        details.chapters_count,
                        cached_data,
                    ).await;
                    if !chapters_with_titles.is_empty() {
                        details.chapters = chapters_with_titles;
                    }
                } else if details.chapters.is_empty() {
                    // If no chapters found, try fetching from separate chapters page
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
            Err(_) => None,
        }
    }

    /// Fetch chapter titles incrementally - only fetches pages with new chapters
    async fn fetch_chapter_titles_incremental(
        &self,
        source: &SourceWithConfig,
        book_id: &str,
        parser: &ConfigurableParser,
        new_chapters_count: i32,
        cached_data: Option<(i32, std::collections::HashMap<i32, String>)>,
    ) -> Vec<crate::models::ParsedChapterInfo> {
        let template = match &source.config.details.chapter_id_template {
            Some(t) => t.clone(),
            None => return vec![],
        };

        let chapters_per_page = 100;

        // Determine what we already have cached
        let (cached_count, all_titles) = cached_data.unwrap_or((0, std::collections::HashMap::new()));

        // If no new chapters, just return what we have
        if new_chapters_count <= cached_count && !all_titles.is_empty() {
            println!("[CHAPTERS] No new chapters (cached: {}, new: {}), using cache", cached_count, new_chapters_count);
            return (1..=new_chapters_count)
                .map(|n| crate::models::ParsedChapterInfo {
                    id: template.replace("{n}", &n.to_string()),
                    title: all_titles.get(&n).cloned().unwrap_or_else(|| format!("Chapter {}", n)),
                    date: None,
                })
                .collect();
        }

        // Calculate which pages we need to fetch
        // Only fetch pages that contain chapters we don't have
        let first_new_chapter = cached_count + 1;
        let start_page = ((first_new_chapter - 1) / chapters_per_page) + 1;
        let total_pages = ((new_chapters_count - 1) / chapters_per_page) + 1;

        println!(
            "[CHAPTERS] Incremental fetch: cached={}, new={}, fetching pages {}-{}",
            cached_count, new_chapters_count, start_page, total_pages
        );

        let mut all_titles = all_titles;

        // Fetch only the pages with new chapters
        for page in start_page..=total_pages {
            let chapters_url = if page == 1 {
                format!("{}/{}/chapters", source.books_url.trim_end_matches('/'), book_id)
            } else {
                format!("{}/{}/chapters?page={}", source.books_url.trim_end_matches('/'), book_id, page)
            };

            println!("[CHAPTERS] Fetching page {}: {}", page, chapters_url);

            if let Ok(resp) = self.client.get(&chapters_url).send().await {
                if let Ok(chapters_html) = resp.text().await {
                    if let Ok(chapters) = parser.parse_chapters_only(&chapters_html) {
                        // Calculate what chapters should be on this page
                        let start_ch = (page - 1) * chapters_per_page + 1;

                        // Map the chapters from this page to their actual chapter numbers
                        for (idx, ch) in chapters.into_iter().enumerate() {
                            let actual_chapter_num = start_ch + idx as i32;
                            if actual_chapter_num <= new_chapters_count {
                                // Only add if we don't have it cached
                                if !all_titles.contains_key(&actual_chapter_num) {
                                    println!("[CHAPTERS] Adding chapter {}: {}", actual_chapter_num, ch.title);
                                    all_titles.insert(actual_chapter_num, ch.title);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Generate chapters with template IDs but actual titles
        (1..=new_chapters_count)
            .map(|n| crate::models::ParsedChapterInfo {
                id: template.replace("{n}", &n.to_string()),
                title: all_titles.get(&n).cloned().unwrap_or_else(|| format!("Chapter {}", n)),
                date: None,
            })
            .collect()
    }

    /// Get book metadata only (no chapters) - returns immediately
    pub async fn get_book_metadata_only(
        &self,
        source: &SourceWithConfig,
        book_id: String,
    ) -> Option<ParsedBookDetails> {
        let parser = ConfigurableParser::new(source.config.clone());
        let url = format!("{}/{}", source.books_url.trim_end_matches('/'), book_id);

        let resp = self.client.get(&url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        match parser.parse_book_details(&html, book_id.clone()) {
            Ok(mut details) => {
                // Return metadata only, chapters will be streamed separately
                details.chapters = vec![];
                Some(details)
            }
            Err(_) => None,
        }
    }

    /// Stream chapters in background - call after metadata is loaded
    pub async fn stream_chapters(
        &self,
        source: &SourceWithConfig,
        book_id: &str,
        chapters_count: i32,
        chapters_tx: std::sync::mpsc::Sender<Vec<crate::models::ParsedChapterInfo>>,
    ) {
        let parser = ConfigurableParser::new(source.config.clone());

        // If using chapter template, stream from paginated chapters pages
        if source.config.details.chapter_id_template.is_some() && chapters_count > 0 {
            self.fetch_chapter_titles_streaming(
                source,
                book_id,
                &parser,
                chapters_count,
                chapters_tx,
            ).await;
        } else {
            // Try fetching from separate chapters page
            let chapters_url = format!(
                "{}/{}/chapters",
                source.books_url.trim_end_matches('/'),
                book_id
            );

            if let Ok(resp) = self.client.get(&chapters_url).send().await {
                if let Ok(chapters_html) = resp.text().await {
                    if let Ok(chapters) = parser.parse_chapters_only(&chapters_html) {
                        let _ = chapters_tx.send(chapters);
                    }
                }
            }
        }
    }

    /// Fetch chapter titles with streaming - sends each page of chapters as they're fetched
    async fn fetch_chapter_titles_streaming(
        &self,
        source: &SourceWithConfig,
        book_id: &str,
        parser: &ConfigurableParser,
        chapters_count: i32,
        chapters_tx: std::sync::mpsc::Sender<Vec<crate::models::ParsedChapterInfo>>,
    ) {
        let template = match &source.config.details.chapter_id_template {
            Some(t) => t.clone(),
            None => return,
        };

        let chapters_per_page = 100;
        let total_pages = ((chapters_count - 1) / chapters_per_page) + 1;

        println!(
            "[CHAPTERS STREAMING] Total chapters: {}, pages: {}",
            chapters_count, total_pages
        );

        for page in 1..=total_pages {
            let chapters_url = if page == 1 {
                format!("{}/{}/chapters", source.books_url.trim_end_matches('/'), book_id)
            } else {
                format!("{}/{}/chapters?page={}", source.books_url.trim_end_matches('/'), book_id, page)
            };

            println!("[CHAPTERS STREAMING] Fetching page {}: {}", page, chapters_url);

            if let Ok(resp) = self.client.get(&chapters_url).send().await {
                if let Ok(chapters_html) = resp.text().await {
                    if let Ok(chapters) = parser.parse_chapters_only(&chapters_html) {
                        // Calculate what chapters should be on this page
                        let start_ch = (page - 1) * chapters_per_page + 1;
                        let end_ch = (page * chapters_per_page).min(chapters_count);

                        println!("[CHAPTERS STREAMING] Page {} should have chapters {}-{}, got {} chapters from HTML",
                                 page, start_ch, end_ch, chapters.len());

                        // Map the chapters from this page to their actual chapter numbers
                        let page_chapters: Vec<crate::models::ParsedChapterInfo> = chapters
                            .into_iter()
                            .enumerate()
                            .filter_map(|(idx, ch)| {
                                let actual_chapter_num = start_ch + idx as i32;
                                if actual_chapter_num <= chapters_count {
                                    Some(crate::models::ParsedChapterInfo {
                                        id: template.replace("{n}", &actual_chapter_num.to_string()),
                                        title: ch.title,
                                        date: ch.date,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        println!("[CHAPTERS STREAMING] Sending {} chapters (numbers {}-{})",
                                 page_chapters.len(), start_ch, start_ch + page_chapters.len() as i32 - 1);

                        // Send this page of chapters
                        if chapters_tx.send(page_chapters).is_err() {
                            // Receiver dropped, stop fetching
                            println!("[CHAPTERS STREAMING] Receiver dropped, stopping");
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn load_home(&self, source: &SourceWithConfig) -> Option<Vec<HomeSection>> {
        let parser = ConfigurableParser::new(source.config.clone());
        let resp = self.client.get(&source.discover_url).send().await.ok()?;
        let html = resp.text().await.ok()?;
        parser.parse_home(&html, &source.url).ok()
    }

    /// Load home page with streaming - sends each section via channel as it's parsed
    /// Returns the total number of sections found
    pub async fn load_home_streaming(
        &self,
        source: &SourceWithConfig,
        section_tx: std::sync::mpsc::Sender<HomeSection>,
    ) -> Option<usize> {
        let parser = ConfigurableParser::new(source.config.clone());
        let resp = self.client.get(&source.discover_url).send().await.ok()?;
        let html = resp.text().await.ok()?;
        parser.parse_home_streaming(&html, &source.url, section_tx).ok()
    }

    pub async fn get_chapter_from_web(
        &self,
        source: &SourceWithConfig,
        book_id: String,
        chapter_id: String,
    ) -> Option<ParsedChapter> {
        let parser = ConfigurableParser::new(source.config.clone());
        let url = format!(
            "{}/{}/{}",
            source.books_url.trim_end_matches('/'),
            book_id,
            chapter_id
        );
        println!("Fetching chapter from URL: {}", url);
        let resp = self.client.get(&url).send().await.ok()?;
        let status = resp.status();
        println!("Response status: {}", status);
        let html = resp.text().await.ok()?;
        println!("Got HTML, length: {}", html.len());
        let result = parser.parse_chapter_content(&html);
        match &result {
            Ok(chapter) => println!("Parsed chapter: title={}, content_len={}", chapter.title, chapter.content.len()),
            Err(e) => println!("Parse error: {}", e),
        }
        result.ok()
    }

    /// Search books using the source's search configuration
    pub async fn search_books(
        &self,
        source: &SourceWithConfig,
        keyword: &str,
    ) -> Option<Vec<SearchResult>> {
        let search_config = source.config.search.as_ref()?;

        // URL-encode the keyword
        let encoded_keyword = urlencoding::encode(keyword);
        let url = search_config.url_pattern.replace("{keyword}", &encoded_keyword);

        println!("[SEARCH] URL: {}", url);

        let resp = self.client.get(&url).send().await.ok()?;
        let status = resp.status();
        println!("[SEARCH] Response status: {}", status);

        if search_config.response_type == "json" {
            self.parse_json_search_results(resp, search_config).await
        } else {
            // HTML parsing would go here if needed
            self.parse_html_search_results(resp, source, search_config).await
        }
    }

    async fn parse_json_search_results(
        &self,
        resp: reqwest::Response,
        search_config: &crate::models::SearchConfig,
    ) -> Option<Vec<SearchResult>> {
        let json: serde_json::Value = resp.json().await.ok()?;

        // Navigate to results array using the path (e.g., "data" or "results.items")
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

                // Build cover URL (may need base URL prepended)
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

                println!("[SEARCH] Book: {} -> Cover URL: {}", title, cover_url);

                // Optional: chapters count
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

        println!("[SEARCH] Found {} results", results.len());
        Some(results)
    }

    async fn parse_html_search_results(
        &self,
        resp: reqwest::Response,
        source: &SourceWithConfig,
        search_config: &crate::models::SearchConfig,
    ) -> Option<Vec<SearchResult>> {
        use scraper::{Html, Selector};
        use regex::Regex;

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
                // Extract ID from link href
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

                let title = item.select(&title_sel).next()?.text().collect::<String>().trim().to_string();

                let cover_url = cover_sel.as_ref().and_then(|sel| {
                    let img = item.select(sel).next()?;
                    let src = img.value().attr("src").or_else(|| img.value().attr("data-src"))?;
                    if src.starts_with("http") {
                        Some(src.to_string())
                    } else if !search_config.cover_base_url.is_empty() {
                        Some(format!("{}{}", search_config.cover_base_url, src))
                    } else {
                        Some(format!("{}{}", source.url, src))
                    }
                }).unwrap_or_default();

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

        println!("[SEARCH] Found {} results from HTML", results.len());
        Some(results)
    }
}
