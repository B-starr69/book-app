use crate::models::{
    HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult, SourceConfig,
};
use anyhow::{anyhow, Context, Result};
use rquickjs::{Function, Runtime, Value};
use serde::de::DeserializeOwned;

/// A parser that uses JavaScript via QuickJS for sources that need custom logic.
pub struct ConfigurableParser {
    source_id: String,
    config: SourceConfig,
    _runtime: Runtime,
}

impl ConfigurableParser {
    pub fn new(source_id: String, config: SourceConfig) -> Self {
        let runtime = Runtime::new().unwrap();
        Self {
            source_id,
            config,
            _runtime: runtime,
        }
    }

    /// Convert a QuickJS Value to a Rust type via JSON.stringify + serde_json.
    /// This is more reliable than rquickjs-serde which can mishandle arrays.
    fn js_value_to_rust<'js, T: DeserializeOwned>(
        ctx: &rquickjs::Ctx<'js>,
        value: Value<'js>,
    ) -> Result<T> {
        let globals = ctx.globals();
        let json_obj: rquickjs::Object = globals
            .get("JSON")
            .map_err(|e| anyhow::anyhow!("Failed to get JSON object: {}", e))?;
        let stringify_fn: Function = json_obj
            .get("stringify")
            .map_err(|e| anyhow::anyhow!("Failed to get JSON.stringify: {}", e))?;
        let json_str: String = stringify_fn
            .call((value,))
            .map_err(|e| anyhow::anyhow!("JSON.stringify failed: {}", e))?;
        serde_json::from_str(&json_str).with_context(|| {
            format!(
                "JSON deserialization failed into Rust type. Raw JSON output was:\n{}",
                json_str
            )
        })
    }

    /// Helper to load script content, run QuickJS environment, and execute the specified JS parser function.
    fn execute_js_parser<T: DeserializeOwned>(&self, function_name: &str, arg: &str) -> Result<T> {
        let script_path = self.config.script_path.as_deref().unwrap_or("index.js");
        let script = crate::storage::load_script_content(&self.source_id, script_path)
            .map_err(|e| anyhow!("Failed to load script content for source '{}' from '{}': {}", self.source_id, script_path, e))?;

        let rt = Runtime::new().map_err(|e| anyhow!("Failed to create JS runtime: {e}"))?;
        let ctx = rquickjs::Context::full(&rt)
            .map_err(|e| anyhow!("Failed to create JS context: {e}"))?;

        ctx.with(|ctx| {
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow!("JS evaluation failed for script '{}': {e}", script_path))?;

            let globals = ctx.globals();
            let parse_fn: Function = globals
                .get(function_name)
                .map_err(|e| anyhow!("Failed to find JS function '{function_name}' in script '{}': {e}", script_path))?;

            let value: Value = parse_fn
                .call((arg,))
                .map_err(|e| anyhow!("JS function '{function_name}' execution failed: {e}"))?;

            let result: T = Self::js_value_to_rust(&ctx, value)
                .map_err(|e| anyhow!("Failed to deserialize JS result of '{function_name}': {e}"))?;

            Ok(result)
        })
    }

    /// Parse the home/discover page using a JS function.
    pub fn parse_home(&self, html: &str, _base_url: &str) -> Result<Vec<HomeSection>> {
        self.execute_js_parser("parseHome", html)
    }

    /// Parse book details page using a JS function.
    pub fn parse_book_details(&self, html: &str, _book_id: String) -> Result<ParsedBookDetails> {
        self.execute_js_parser("parseBookDetails", html)
    }

    /// Parse just the chapters list from a dedicated chapters page.
    pub fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>> {
        self.execute_js_parser("parseChapters", html)
    }

    /// Parse chapter content page using a JS function.
    pub fn parse_chapter_content(&self, html: &str) -> Result<ParsedChapter> {
        self.execute_js_parser("parseChapterContent", html)
    }

    /// Parse search results using a JS function.
    pub fn parse_search_results(&self, payload: &str) -> Result<Vec<SearchResult>> {
        self.execute_js_parser("parseSearch", payload)
    }
}
