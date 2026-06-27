use crate::models::SourceConfig;
use crate::platform;
use anyhow::{anyhow, Result};
use rquickjs::{Context, Function, Runtime};
use std::path::PathBuf;

/// A fetcher that uses JavaScript via QuickJS for dynamic content fetching.
pub struct ConfigurableFetcher {
    config: SourceConfig,
}

impl ConfigurableFetcher {
    pub fn new(config: SourceConfig) -> Self {
        Self { config }
    }

    /// Load the source JavaScript extension script from disk.
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

    /// Expose native helper functions (like `fetchUrl`) to the QuickJS execution context.
    fn register_helpers(&self, ctx: &rquickjs::Ctx<'_>) -> Result<()> {
        let globals = ctx.globals();
        globals.set(
            "fetchUrl",
            rquickjs::Function::new(ctx.clone(), |url: String| -> Result<String, rquickjs::Error> {
                let client = reqwest::blocking::Client::builder()
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                    .build()
                    .map_err(|e| rquickjs::Error::new_into_js_message("reqwest", "std::string::String", e.to_string()))?;
                let resp = client.get(&url)
                    .send()
                    .map_err(|e| rquickjs::Error::new_into_js_message("reqwest", "std::string::String", e.to_string()))?;
                let text = resp.text()
                    .map_err(|e| rquickjs::Error::new_into_js_message("reqwest", "std::string::String", e.to_string()))?;
                Ok(text)
            })?,
        )?;
        Ok(())
    }

    pub fn fetch_home(&self) -> Result<String> {
        let function_name = "fetchHome";
        self.call_js_fetch_zero_args(function_name)
    }

    pub fn fetch_details(&self, book_id: &str) -> Result<String> {
        let function_name = "fetchBookDetails";
        self.call_js_fetch_one_arg(function_name, book_id)
    }

    pub fn fetch_chapters_list(&self, book_id: &str) -> Result<String> {
        let function_name = "fetchChaptersList";
        self.call_js_fetch_one_arg(function_name, book_id)
    }

    pub fn fetch_chapter_content(&self, book_id: &str, chapter_id: &str) -> Result<String> {
        let function_name = "fetchChapterContent";
        self.call_js_fetch_two_args(function_name, book_id, chapter_id)
    }

    pub fn fetch_search(&self, keyword: &str, genre: Option<&str>) -> Result<String> {
        let search_config = self
            .config
            .search
            .as_ref()
            .ok_or_else(|| anyhow!("Search capability is not configured for this source"))?;
        let function_name = "fetchSearch";

        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;
        ctx.with(|ctx| {
            self.register_helpers(&ctx)?;
            let script = self.get_script_content();
            ctx.eval::<(), _>(script.as_bytes())?;
            let globals = ctx.globals();
            let func: Function = globals.get(function_name)?;

            let genre_val = genre.unwrap_or("");
            let val: String = func.call((keyword, genre_val))?;
            Ok(val)
        })
    }

    fn call_js_fetch_zero_args(&self, function_name: &str) -> Result<String> {
        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;
        ctx.with(|ctx| {
            self.register_helpers(&ctx)?;
            let script = self.get_script_content();
            ctx.eval::<(), _>(script.as_bytes())?;
            let globals = ctx.globals();
            let func: Function = globals.get(function_name)?;
            let val: String = func.call(())?;
            Ok(val)
        })
    }

    fn call_js_fetch_one_arg(&self, function_name: &str, arg: &str) -> Result<String> {
        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;
        ctx.with(|ctx| {
            self.register_helpers(&ctx)?;
            let script = self.get_script_content();
            ctx.eval::<(), _>(script.as_bytes())?;
            let globals = ctx.globals();
            let func: Function = globals.get(function_name)?;
            let val: String = func.call((arg,))?;
            Ok(val)
        })
    }

    fn call_js_fetch_two_args(
        &self,
        function_name: &str,
        arg1: &str,
        arg2: &str,
    ) -> Result<String> {
        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;
        ctx.with(|ctx| {
            self.register_helpers(&ctx)?;
            let script = self.get_script_content();
            ctx.eval::<(), _>(script.as_bytes())?;
            let globals = ctx.globals();
            let func: Function = globals.get(function_name)?;
            let val: String = func.call((arg1, arg2))?;
            Ok(val)
        })
    }
}
