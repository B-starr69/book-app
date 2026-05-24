use crate::models::{
    HomeSection, LayoutMapping, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult,
    SectionLayout, SourceConfig,
};
use regex::Regex;
use scraper::{Html, Selector};

use rquickjs::{Context, Function, Runtime, Value};
use anyhow::Result;

/// A parser that uses configurable CSS selectors from the database
pub struct ConfigurableParser {
    config: SourceConfig,
    runtime: Runtime,
}

impl ConfigurableParser {
    pub fn new(config: SourceConfig) -> Self {
        let runtime = Runtime::new().unwrap();
        Self { config, runtime }
    }

    /// Parse the home/discover page using configured selectors
    pub fn parse_home(&self, html: &str, _base_url: &str) -> Result<Vec<HomeSection>> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.home.script.as_deref().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self.config.home.js_function.as_deref().unwrap_or("parseHome");
            let parse_home_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_home_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let sections: Vec<HomeSection> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(sections)
        })
    }

    /// Parse home page with streaming - sends each section via channel as it's parsed
    /// Returns the total count of sections found
    pub fn parse_home_streaming(
        &self,
        html: &str,
        _base_url: &str,
        section_tx: std::sync::mpsc::Sender<HomeSection>,
    ) -> Result<usize> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.home.script.as_deref().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self.config.home.js_function.as_deref().unwrap_or("parseHome");
            let parse_home_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_home_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let sections: Vec<HomeSection> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            for section in sections {
                if section_tx.send(section).is_err() {
                    // Receiver has been dropped
                    break;
                }
            }

            Ok(0) // Return value doesn't matter much here
        })
    }

    /// Parse book details page using configured selectors
    pub fn parse_book_details(&self, html: &str, book_id: String) -> Result<ParsedBookDetails> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.details.script.as_deref().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .details
                .js_function
                .as_deref()
                .unwrap_or("parseBookDetails");
            let parse_details_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_details_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let mut details: ParsedBookDetails = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(details)
        })
    }

    /// Parse just the chapters list from a dedicated chapters page
    pub fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.details.script.as_deref().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .details
                .js_function
                .as_deref()
                .unwrap_or("parseChapters");
            let parse_chapters_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_chapters_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let chapters: Vec<ParsedChapterInfo> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(chapters)
        })
    }

    /// Parse chapter content page using configured selectors
    pub fn parse_chapter_content(&self, html: &str) -> Result<ParsedChapter> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.chapter.script.as_deref().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .chapter
                .js_function
                .as_deref()
                .unwrap_or("parseChapterContent");
            let parse_content_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_content_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let chapter: ParsedChapter = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(chapter)
        })
    }

    pub fn parse_search_results(&self, payload: &str) -> Result<Vec<SearchResult>> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self
                .config
                .search
                .as_ref()
                .and_then(|search| search.script.as_deref())
                .unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .search
                .as_ref()
                .and_then(|search| search.js_function.as_deref())
                .unwrap_or("parseSearch");
            let parse_search_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_search_fn.call((payload,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let results: Vec<SearchResult> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(results)
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
