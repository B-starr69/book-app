use crate::models::{
    HomeSection, ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult, SourceConfig,
};
use crate::platform;
use anyhow::{anyhow, Context, Result};
use rquickjs::{Function, Runtime, Value};
use serde::de::DeserializeOwned;
use std::path::PathBuf;
/// A parser that uses JavaScript via QuickJS for sources that need custom logic.
pub struct ConfigurableParser {
    config: SourceConfig,
    _runtime: Runtime,
}

impl ConfigurableParser {
    pub fn new(config: SourceConfig) -> Self {
        let runtime = Runtime::new().unwrap();
        Self {
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
    fn get_script_content(&self) -> String {
        if let Some(ref path) = self.config.script_path {
            let path_buf = PathBuf::from(path);
            let mut resolved = if path_buf.is_absolute() {
                path_buf.clone()
            } else if path_buf.exists() {
                path_buf.clone()
            } else {
                platform::get_app_data_dir().join(&path_buf)
            };

            if !resolved.exists() && path_buf.is_relative() {
                if let Ok(cwd) = std::env::current_dir() {
                    let mut current = cwd.as_path();
                    while let Some(parent) = current.parent() {
                        let candidate = parent.join(&path_buf);
                        if candidate.exists() {
                            resolved = candidate;
                            break;
                        }
                        current = parent;
                    }
                }
            }
            std::fs::read_to_string(resolved).unwrap_or_default()
        } else {
            String::new()
        }
    }
    /// Parse the home/discover page using a JS function.
    pub fn parse_home(&self, html: &str, _base_url: &str) -> Result<Vec<HomeSection>> {
        let rt = Runtime::new().map_err(|e| anyhow!("Failed to create JS runtime: {e}"))?;
        let ctx = rquickjs::Context::full(&rt)
            .map_err(|e| anyhow!("Failed to create JS context: {e}"))?;

        ctx.with(|ctx| {
            let script = Self::get_script_content(&self);
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow!("JS evaluation failed: {e}"))?;

            let globals = ctx.globals();
            let function_name = "parseHome";
            let parse_home_fn: Function = globals
                .get(function_name)
                .map_err(|e| anyhow!("Failed to find JS function '{function_name}': {e}"))?;

            let value: Value = parse_home_fn
                .call((html,))
                .map_err(|e| anyhow!("JS function '{function_name}' execution failed: {e}"))?;

            // This will now automatically include the JSON dump if mapping fails
            let sections: Vec<HomeSection> = Self::js_value_to_rust(&ctx, value)?;

            Ok(sections)
        })
    }

    /// Parse book details page using a JS function.
    pub fn parse_book_details(&self, html: &str, _book_id: String) -> Result<ParsedBookDetails> {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.clone().script_path.unwrap();
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = "parseBookDetails";
            let parse_details_fn: Function = globals
                .get(function_name)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_details_fn
                .call((html,))
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let details: ParsedBookDetails = Self::js_value_to_rust(&ctx, value)?;

            Ok(details)
        })
    }

    /// Parse just the chapters list from a dedicated chapters page.
    pub fn parse_chapters_only(&self, html: &str) -> Result<Vec<ParsedChapterInfo>> {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.clone().script_path.unwrap();
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = "parseChapters";
            let parse_chapters_fn: Function = globals
                .get(function_name)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_chapters_fn
                .call((html,))
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let chapters: Vec<ParsedChapterInfo> = Self::js_value_to_rust(&ctx, value)?;

            Ok(chapters)
        })
    }

    /// Parse chapter content page using a JS function.
    pub fn parse_chapter_content(&self, html: &str) -> Result<ParsedChapter> {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.clone().script_path.unwrap();
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = "parseChapterContent";
            let parse_content_fn: Function = globals
                .get(function_name)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_content_fn
                .call((html,))
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let chapter: ParsedChapter = Self::js_value_to_rust(&ctx, value)?;

            Ok(chapter)
        })
    }

    /// Parse search results using a JS function.
    pub fn parse_search_results(&self, payload: &str) -> Result<Vec<SearchResult>> {
        let rt = Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let script = self.config.clone().script_path.unwrap();
            ctx.eval::<(), _>(script.as_bytes())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let globals = ctx.globals();
            let function_name = "parseSearch";
            let parse_search_fn: Function = globals
                .get(function_name)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let value: Value = parse_search_fn
                .call((payload,))
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let results: Vec<SearchResult> = Self::js_value_to_rust(&ctx, value)?;

            Ok(results)
        })
    }
}
