use crate::models::{
    HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SourceConfig, Strategy,JsonSearchMapping
};
use crate::parser_utils::{
    determine_layout, extract_all_text, extract_attr, extract_id_from_pattern, extract_text,
    parse_selector, sanitize_chapter_html,
};
use regex::Regex;
use scraper::Html;

/// Native Rust parser for source configs that use CSS selectors.
pub struct NativeParser {
    config: SourceConfig,
}

impl NativeParser {
    pub fn new(config: SourceConfig) -> Self {
        Self { config }
    }

    pub fn parse_home(&self, html: &str) -> Result<Vec<HomeSection>, String> {
        let document = Html::parse_document(html);
        let selectors = match &self.config.home.parse {
            crate::models::Strategy::Rust(ref s) => s,
            _ => return Err("Home selectors strategy is JS, cannot parse natively".to_string()),
        };

        let section_sel = parse_selector(&selectors.section)?;
        let header_sel = parse_selector(&selectors.header)?;
        let item_sel = parse_selector(&selectors.item)?;
        let link_sel = parse_selector(&selectors.link)?;
        let book_id_regex = Regex::new(&selectors.book_id_pattern)
            .map_err(|e| format!("Invalid book_id_pattern: {}", e))?;

        let sections: Vec<HomeSection> = document
            .select(&section_sel)
            .filter_map(|section_node| {
                // 1. Extract and clean the section header title string
                let title = section_node
                    .select(&header_sel)
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string();

                let layout = determine_layout(&title, &selectors.layout_mapping);

                // 2. Map and parse child element items into string item identifiers
                let books: Vec<String> = section_node
                    .select(&item_sel)
                    .filter_map(|item| {
                        // Find the structural anchor/link element inside the layout item context
                        let link_node = item.select(&link_sel).next()?;
                        let href = link_node.value().attr(&selectors.href_attr)?;

                        // Extract the uniquely identifiable ID segment via pattern string matching
                        let id = extract_id_from_pattern(href, &book_id_regex)?;

                        // NOTE: If you need to map images or save minimal details to a side-cache db,
                        // you would extract `cover_url = item.select(...)` here.

                        Some(id)
                    })
                    .collect();

                // Prevent returning empty category lists
                if books.is_empty() {
                    return None;
                }

                Some(HomeSection {
                    title,
                    layout,
                    books, // Successfully maps directly to Vec<String>
                })
            })
            .collect();

        Ok(sections)
    }

    pub fn parse_book_details(&self, html: &str, _id: String) -> Result<ParsedBookDetails, String> {
        let document = Html::parse_document(html);
        let sel = match &self.config.details.parse {
            crate::models::Strategy::Rust(ref s) => s,
            _ => return Err("Details strategy is JS, cannot parse natively".to_string()),
        };

        let title = extract_text(&document, &sel.title).unwrap_or("Unknown Title".to_string());
        let author = extract_text(&document, &sel.author).unwrap_or("Unknown Author".to_string());

        let cover_url = extract_attr(&document, &sel.cover, &sel.cover_attr)
            .or_else(|| {
                sel.cover_attr_alt
                    .as_ref()
                    .and_then(|alt| extract_attr(&document, &sel.cover, alt))
            })
            .unwrap_or_default();

        let rating = extract_text(&document, &sel.rating)
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(0.0);

        let status = extract_text(&document, &sel.status).unwrap_or("Unknown".to_string());

        let chapters_count = extract_text(&document, &sel.chapters_count)
            .and_then(|s| {
                s.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<i32>()
                    .ok()
            })
            .unwrap_or(0);

        let genres = extract_all_text(&document, &sel.genres);
        let summary = extract_text(&document, &sel.summary).unwrap_or_default();

        Ok(ParsedBookDetails {
            title,
            author,
            cover_url,
            rating,
            status,
            chapters_count,
            genres,
            summary,
        })
    }

    pub fn parse_chapters_list(&self, html: &str) -> Result<Vec<ParsedChapterInfo>, String> {
        let document = Html::parse_document(html);

        // 1. Extract the declarative selector configurations
        let sel = match &self.config.chapters_list.parse {
            Strategy::Rust(ref s) => s,
            _ => return Err("Chapters list strategy is not native Rust".to_string()),
        };

        // 2. Pre-compile Regex and Selectors safely upfront to eliminate silent runtime failures
        let chapter_id_regex = Regex::new(&sel.id_regex)
            .map_err(|e| format!("Invalid chapter id_regex layout pattern: {}", e))?;

        let list_item_selector = scraper::Selector::parse(&sel.chapter_list)
            .map_err(|e| format!("Invalid structural chapter_list CSS selector: {:?}", e))?;

        let title_selector = scraper::Selector::parse(&sel.title)
            .map_err(|e| format!("Invalid structural title CSS selector: {:?}", e))?;

        let date_selector = scraper::Selector::parse(&sel.date)
            .map_err(|e| format!("Invalid structural date CSS selector: {:?}", e))?;

        // 3. Document DOM Traversal Iteration Block
        let chapters = document
            .select(&list_item_selector)
            .filter_map(|el| {
                // Extract the target link attribute using config instead of hardcoded "href"
                let href = el.value().attr(&sel.id_attr)?;
                let id = extract_id_from_pattern(href, &chapter_id_regex)?;

                // Extract the Title: Try config sub-selector target first, fallback to node text stream
                let title = el
                    .select(&title_selector)
                    .next()
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .or_else(|| {
                        let text = el.text().collect::<String>();
                        let clean = text.trim().to_string();
                        if clean.is_empty() {
                            None
                        } else {
                            Some(clean)
                        }
                    })?;

                // Extract the Date: Checks inner text node or fallback tag attributes (like datetime="")
                let date = el
                    .select(&date_selector)
                    .next()
                    .map(|e| {
                        if let Some(ref attr_target) = sel.date_attr {
                            e.value().attr(attr_target).map(|s| s.trim().to_string())
                        } else {
                            Some(e.text().collect::<String>().trim().to_string())
                        }
                    })
                    .flatten().unwrap().parse::<i64>().unwrap(); // Flattens the nested Option<Option<String>> cleanly

                Some(ParsedChapterInfo { id, title, date:Some(date) })
            })
            .collect();

        Ok(chapters)
    }

    pub fn parse_chapter_content(&self, html: &str) -> Result<ParsedChapter, String> {
        let document = Html::parse_document(html);
        let sel = match &self.config.chapter.parse {
            crate::models::Strategy::Rust(ref s) => s,
            _ => return Err("Chapter strategy is JS, cannot parse natively".to_string()),
        };

        let title = extract_text(&document, &sel.title).unwrap_or("Unknown Chapter".to_string());

        let content = if let Ok(content_sel) = scraper::Selector::parse(&sel.content) {
            document
                .select(&content_sel)
                .next()
                .map(|el| el.inner_html())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let content = sanitize_chapter_html(&content);

        Ok(ParsedChapter { title, content })
    }

    pub async fn parse_json_search_results(
        &self,
        resp: reqwest::Response,
        mapping: JsonSearchMapping,
        json_results_path: &str,
    ) -> Option<Vec<String>> {
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

        let results: Vec<String> = results_array
            .iter()
            .filter_map(|item| {
                let id = item.get(&mapping.id_key)?.as_str()?.to_string();

                Some(id)
            })
            .collect();

        Some(results)
    }

    pub async fn parse_html_search_results(
        &self,
        resp: reqwest::Response,
        item_selector: &str,
        mapping: &crate::models::HtmlSearchMapping,
    ) -> Option<Vec<String>> {
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

        let results: Vec<String> = document
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

                Some(id)
            })
            .collect();

        Some(results)
    }
}
