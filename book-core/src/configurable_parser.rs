use crate::models::{
    HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult,
    SourceConfig,
};
use crate::parser_utils::sanitize_chapter_html;

use rquickjs::{Context, Function, Runtime, Value};
use anyhow::Result;

/// A parser that uses JavaScript via QuickJS for sources that need custom logic.
pub struct ConfigurableParser {
    config: SourceConfig,
    _runtime: Runtime,
}

impl ConfigurableParser {
    pub fn new(config: SourceConfig) -> Self {
        let runtime = Runtime::new().unwrap();
        Self { config, _runtime: runtime }
    }

    /// Parse the home/discover page using a JS function.
    pub fn parse_home(&self, html: &str, _base_url: &str) -> Result<Vec<HomeSection>> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.home.js_script().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self.config.home.js_function().unwrap_or("parseHome");
            let parse_home_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_home_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let sections: Vec<HomeSection> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(sections)
        })
    }

    /// Parse book details page using a JS function.
    pub fn parse_book_details(&self, html: &str, _book_id: String) -> Result<ParsedBookDetails> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.details.js_script().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .details
                .js_function()
                .unwrap_or("parseBookDetails");
            let parse_details_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_details_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let details: ParsedBookDetails = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(details)
        })
    }

    /// Parse just the chapters list from a dedicated chapters page.
    pub fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.details.js_script().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .details
                .js_function()
                .unwrap_or("parseChapters");
            let parse_chapters_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_chapters_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let chapters: Vec<ParsedChapterInfo> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(chapters)
        })
    }

    /// Parse chapter content page using a JS function.
    pub fn parse_chapter_content(&self, html: &str) -> Result<ParsedChapter> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.chapter.js_script().unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .chapter
                .js_function()
                .unwrap_or("parseChapterContent");
            let parse_content_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_content_fn.call((html,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let mut chapter: ParsedChapter = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            chapter.content = sanitize_chapter_html(&chapter.content);

            Ok(chapter)
        })
    }

    /// Parse search results using a JS function.
    pub fn parse_search_results(&self, payload: &str) -> Result<Vec<SearchResult>> {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self
                .config
                .search
                .as_ref()
                .and_then(|search| search.js_script())
                .unwrap_or("");
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = self
                .config
                .search
                .as_ref()
                .and_then(|search| search.js_function())
                .unwrap_or("parseSearch");
            let parse_search_fn: Function = globals.get(function_name).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_search_fn.call((payload,)).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let results: Vec<SearchResult> = rquickjs_serde::from_value(value).map_err(|e| anyhow::anyhow!(e.to_string()))?;

            Ok(results)
        })
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use crate::parser_utils::extract_id_from_pattern;

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
