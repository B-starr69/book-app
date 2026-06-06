use crate::models::{
    Book, HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo,
    SourceConfig,
};
use crate::parser_utils::{
    determine_layout, extract_attr, extract_all_text, extract_id_from_pattern,
    extract_text, make_absolute_url, parse_selector, sanitize_chapter_html,
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

    pub fn parse_home(&self, html: &str, base_url: &str) -> Result<Vec<HomeSection>, String> {
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

        let cover_sel = if !selectors.cover.is_empty() {
            Some(parse_selector(&selectors.cover)?)
        } else {
            None
        };

        let title_sel = if !selectors.title.is_empty() {
            Some(parse_selector(&selectors.title)?)
        } else {
            None
        };

        let sections = document
            .select(&section_sel)
            .filter_map(|section_node| {
                let title = section_node
                    .select(&header_sel)
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string();

                let layout = determine_layout(&title, &selectors.layout_mapping);

                let books: Vec<Book> = section_node
                    .select(&item_sel)
                    .filter_map(|item| {
                        let link = item.select(&link_sel).next()?;
                        let href = link.value().attr(&selectors.href_attr)?;
                        let id = extract_id_from_pattern(href, &book_id_regex)?;

                        let book_title = title_sel.as_ref().and_then(|sel| {
                            let title_elem = item.select(sel).next()?;
                            if let Some(attr) = &selectors.title_attr {
                                title_elem.value().attr(attr).map(|s| s.to_string())
                            } else {
                                Some(title_elem.text().collect::<String>().trim().to_string())
                            }
                        }).unwrap_or_default();

                        let cover_url = cover_sel.as_ref().and_then(|sel| {
                            let img = item.select(sel).next()?;
                            let raw_url = selectors
                                .cover_attr_alt
                                .as_ref()
                                .and_then(|alt| img.value().attr(alt))
                                .or_else(|| img.value().attr(&selectors.cover_attr));

                            raw_url.map(|s| make_absolute_url(s, base_url))
                        }).unwrap_or_default();

                        Some(Book { id, title: book_title, cover_url, ..Default::default() })
                    })
                    .collect();

                if books.is_empty() {
                    return None;
                }

                Some(HomeSection {
                    title,
                    layout,
                    books,
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

        let chapters = if let Some(ref template) = sel.chapter_id_template {
            (1..=chapters_count)
                .map(|n| ParsedChapterInfo {
                    id: template.replace("{n}", &n.to_string()),
                    title: format!("Chapter {}", n),
                    date: None,
                })
                .collect()
        } else {
            let chapter_id_regex = Regex::new(&sel.chapter_id_pattern)
                .map_err(|e| format!("Invalid chapter_id_pattern: {}", e))?;

            if let Ok(chapter_sel) = scraper::Selector::parse(&sel.chapter_list) {
                document
                    .select(&chapter_sel)
                    .filter_map(|el| {
                        let href = el.value().attr("href")?;
                        let id = extract_id_from_pattern(href, &chapter_id_regex)?;
                        let title = el.text().collect::<String>().trim().to_string();

                        let date = sel.chapter_date_attr.as_ref().and_then(|attr| {
                            el.value().attr(attr).map(|s| s.to_string())
                        }).or_else(|| {
                            sel.chapter_date.as_ref().and_then(|date_sel| {
                                scraper::Selector::parse(date_sel).ok().and_then(|s| {
                                    el.select(&s).next().map(|e| e.text().collect::<String>().trim().to_string())
                                })
                            })
                        });

                        Some(ParsedChapterInfo { id, title, date })
                    })
                    .collect()
            } else {
                vec![]
            }
        };

        Ok(ParsedBookDetails {
            title,
            author,
            cover_url,
            rating,
            status,
            chapters_count,
            genres,
            summary,
            chapters,
        })
    }

    pub fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>, String> {
        let document = Html::parse_document(html);
        let sel = match &self.config.details.parse {
            crate::models::Strategy::Rust(ref s) => s,
            _ => return Err("Details strategy is JS, cannot parse natively".to_string()),
        };

        let chapter_id_regex = Regex::new(&sel.chapter_id_pattern)
            .map_err(|e| format!("Invalid chapter_id_pattern: {}", e))?;

        let chapters = if let Ok(chapter_sel) = scraper::Selector::parse(&sel.chapter_list) {
            document
                .select(&chapter_sel)
                .filter_map(|el| {
                    let href = el.value().attr("href")?;
                    let id = extract_id_from_pattern(href, &chapter_id_regex)?;

                    let title = el
                        .select(&scraper::Selector::parse(".chapter-title, strong.chapter-title").ok()?)
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

                    let date = el
                        .select(&scraper::Selector::parse("time, .chapter-update").ok()?)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .or_else(|| {
                            sel.chapter_date_attr.as_ref().and_then(|attr| {
                                el.value().attr(attr).map(|s| s.to_string())
                            })
                        });

                    Some(ParsedChapterInfo { id, title, date })
                })
                .collect()
        } else {
            vec![]
        };

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

        let date = sel.date.as_ref().and_then(|date_selector| {
            if let Some(attr) = &sel.date_attr {
                extract_attr(&document, date_selector, attr)
            } else {
                extract_text(&document, date_selector)
            }
        });

        let content = sanitize_chapter_html(&content);

        Ok(ParsedChapter {
            title,
            content,
            date,
        })
    }
}
