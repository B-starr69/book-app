use crate::models::{
    BookPreview, HomeSection, LayoutMapping, ParsedBookDetails, ParsedChapter, ParsedChapterInfo,
    SectionLayout, SourceConfig,
};
use regex::Regex;
use scraper::{Html, Selector};

/// A parser that uses configurable CSS selectors from the database
pub struct ConfigurableParser {
    config: SourceConfig,
}

/// Make a URL absolute if it's relative
fn make_absolute_url(url: &str, base_url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    if url.starts_with("//") {
        return format!("https:{}", url);
    }
    if url.starts_with('/') {
        // Extract scheme and host from base URL
        if let Some(pos) = base_url.find("://") {
            let after_scheme = &base_url[pos + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                let origin = &base_url[..pos + 3 + slash_pos];
                return format!("{}{}", origin, url);
            }
        }
        return format!("{}{}", base_url.trim_end_matches('/'), url);
    }
    // Relative URL without leading slash
    format!("{}/{}", base_url.trim_end_matches('/'), url)
}

impl ConfigurableParser {
    pub fn new(config: SourceConfig) -> Self {
        Self { config }
    }

    /// Parse the home/discover page using configured selectors
    pub fn parse_home(&self, html: &str, base_url: &str) -> Result<Vec<HomeSection>, String> {


        let document = Html::parse_document(html);
        let selectors = &self.config.home;


        let section_sel = parse_selector(&selectors.section)?;
        let header_sel = parse_selector(&selectors.header)?;
        let item_sel = parse_selector(&selectors.item)?;
        let link_sel = parse_selector(&selectors.link)?;
        let book_id_regex = Regex::new(&selectors.book_id_pattern)
            .map_err(|e| format!("Invalid book_id_pattern: {}", e))?;

        // Parse optional cover selector
        let cover_sel = if !selectors.cover.is_empty() {
            Some(parse_selector(&selectors.cover)?)
        } else {
            None
        };

        // Parse optional title selector
        let title_sel = if !selectors.title.is_empty() {
            Some(parse_selector(&selectors.title)?)
        } else {
            None
        };

        let _section_count = document.select(&section_sel).count();

        let sections = document
            .select(&section_sel)
            .filter_map(|section_node| {
                // Extract title
                let title = section_node
                    .select(&header_sel)
                    .next()?
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string();


                // Determine layout from config mapping
                let layout = determine_layout(&title, &selectors.layout_mapping);

                // Extract book previews (ID, title, and cover URL)
                let books: Vec<BookPreview> = section_node
                    .select(&item_sel)
                    .filter_map(|item| {
                        let link = item.select(&link_sel).next()?;
                        let href = link.value().attr(&selectors.href_attr)?;
                        let id = extract_id_from_pattern(href, &book_id_regex)?;

                        // Try to extract title
                        let book_title = title_sel.as_ref().and_then(|sel| {
                            let title_elem = item.select(sel).next()?;
                            // Check if we should get from attribute or text content
                            if let Some(attr) = &selectors.title_attr {
                                title_elem.value().attr(attr).map(|s| s.to_string())
                            } else {
                                Some(title_elem.text().collect::<String>().trim().to_string())
                            }
                        }).unwrap_or_default();

                        // Try to extract cover URL
                        // Prefer alt attribute (data-src) for lazy-loaded images
                        let cover_url = cover_sel.as_ref().and_then(|sel| {
                            let img = item.select(sel).next()?;
                            // Try alternative attribute first (data-src for lazy loading)
                            let raw_url = selectors
                                .cover_attr_alt
                                .as_ref()
                                .and_then(|alt| img.value().attr(alt))
                                .or_else(|| img.value().attr(&selectors.cover_attr));

                            raw_url.map(|s| make_absolute_url(s, base_url))
                        }).unwrap_or_default();

                        Some(BookPreview { id, title: book_title, cover_url })
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

    /// Parse home page with streaming - sends each section via channel as it's parsed
    /// Returns the total count of sections found
    pub fn parse_home_streaming(
        &self,
        html: &str,
        base_url: &str,
        section_tx: std::sync::mpsc::Sender<HomeSection>,
    ) -> Result<usize, String> {
        let document = Html::parse_document(html);
        let selectors = &self.config.home;

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

        let mut sections_count = 0;

        for section_node in document.select(&section_sel) {
            // Extract title
            let title = match section_node.select(&header_sel).next() {
                Some(h) => h.text().collect::<String>().trim().to_string(),
                None => continue,
            };

            let layout = determine_layout(&title, &selectors.layout_mapping);

            // Extract book previews
            let books: Vec<BookPreview> = section_node
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

                    Some(BookPreview { id, title: book_title, cover_url })
                })
                .collect();

            if books.is_empty() {
                continue;
            }

            // Send section immediately via channel
            let section = HomeSection { title, layout, books };
            if section_tx.send(section).is_err() {
                // Receiver dropped, stop parsing
                break;
            }
            sections_count += 1;
        }

        Ok(sections_count)
    }

    /// Parse book details page using configured selectors
    pub fn parse_book_details(&self, html: &str, _id: String) -> Result<ParsedBookDetails, String> {


        let document = Html::parse_document(html);
        let sel = &self.config.details;



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
                // Extract first number found in the text
                s.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<i32>()
                    .ok()
            })
            .unwrap_or_else(|| {
                eprintln!("[PARSER] chapters_count selector '{}' returned nothing", sel.chapters_count);
                0
            });

        let genres = extract_all_text(&document, &sel.genres);

        let summary = extract_text(&document, &sel.summary).unwrap_or_default();


        // Parse chapters - use template if available, otherwise parse from HTML
        let chapters = if let Some(ref template) = sel.chapter_id_template {
            // Generate chapters using template and chapters_count
            (1..=chapters_count)
                .map(|n| ParsedChapterInfo {
                    id: template.replace("{n}", &n.to_string()),
                    title: format!("Chapter {}", n),
                    date: None,
                })
                .collect()
        } else {
            // Parse chapters from HTML
            let chapter_id_regex = Regex::new(&sel.chapter_id_pattern)
                .map_err(|e| format!("Invalid chapter_id_pattern: {}", e))?;

            if let Ok(chapter_sel) = Selector::parse(&sel.chapter_list) {
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
                                Selector::parse(date_sel).ok().and_then(|s| {
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

    /// Parse just the chapters list from a dedicated chapters page
    pub fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>, String> {
        let document = Html::parse_document(html);
        let sel = &self.config.details;

        let chapter_id_regex = Regex::new(&sel.chapter_id_pattern)
            .map_err(|e| format!("Invalid chapter_id_pattern: {}", e))?;

        let chapters = if let Ok(chapter_sel) = Selector::parse(&sel.chapter_list) {
            document
                .select(&chapter_sel)
                .filter_map(|el| {
                    let href = el.value().attr("href")?;
                    let id = extract_id_from_pattern(href, &chapter_id_regex)?;

                    // Try to get title from nested elements or from text
                    let title = el.select(&Selector::parse(".chapter-title, strong.chapter-title").ok()?)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .or_else(|| {
                            let text = el.text().collect::<String>();
                            let clean = text.trim().to_string();
                            if clean.is_empty() { None } else { Some(clean) }
                        })?;

                    // Try to get date from nested time element
                    let date = el.select(&Selector::parse("time, .chapter-update").ok()?)
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

    /// Parse chapter content page using configured selectors
    pub fn parse_chapter_content(&self, html: &str) -> Result<ParsedChapter, String> {
        let document = Html::parse_document(html);
        let sel = &self.config.chapter;

        let title = extract_text(&document, &sel.title).unwrap_or("Unknown Chapter".to_string());

        let content = if let Ok(content_sel) = Selector::parse(&sel.content) {
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
                // For meta tags: <meta itemprop="datePublished" content="...">
                extract_attr(&document, date_selector, attr)
            } else {
                extract_text(&document, date_selector)
            }
        });

        Ok(ParsedChapter {
            title,
            content,
            date,
        })
    }
}

// ==================== Helper Functions ====================

fn parse_selector(selector: &str) -> Result<Selector, String> {
    Selector::parse(selector).map_err(|e| format!("Invalid selector '{}': {:?}", selector, e))
}

fn extract_text(document: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    document
        .select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
}

fn extract_all_text(document: &Html, selector: &str) -> Vec<String> {
    Selector::parse(selector)
        .ok()
        .map(|sel| {
            document
                .select(&sel)
                .map(|el| el.text().collect::<String>().trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn extract_attr(document: &Html, selector: &str, attr: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr(attr))
        .map(|s| s.to_string())
}

fn extract_id_from_pattern(href: &str, regex: &Regex) -> Option<String> {
    regex
        .captures(href)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .filter(|s| !s.is_empty())
}

fn determine_layout(title: &str, mappings: &[LayoutMapping]) -> SectionLayout {
    for mapping in mappings {
        if title.to_lowercase().contains(&mapping.title_contains.to_lowercase()) {
            return mapping.layout.clone();
        }
    }
    SectionLayout::Grid // default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_id_from_pattern() {
        let regex = Regex::new(r"/book/([^/?#]+)").unwrap();
        assert_eq!(
            extract_id_from_pattern("/book/my-novel-123", &regex),
            Some("my-novel-123".to_string())
        );
        assert_eq!(
            extract_id_from_pattern("/book/test/chapter-1", &regex),
            Some("test".to_string())
        );
        assert_eq!(extract_id_from_pattern("/other/path", &regex), None);
    }
}
