use crate::models::SourceConfig;
use anyhow::{anyhow, Result};
use rquickjs::{Context, Function, Runtime};

/// A fetcher that uses JavaScript via QuickJS for dynamic content fetching.
pub struct ConfigurableFetcher {
    source_id: String,
    config: SourceConfig,
    // Share a single reqwest client across calls
    client: reqwest::Client,
}

impl ConfigurableFetcher {
    pub fn new(source_id: String, config: SourceConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            source_id,
            config,
            client,
        }
    }

    pub async fn fetch_home(&self) -> Result<String> {
        self.call_js_fetch("fetchHome", |func| func.call(())).await
    }

    pub async fn fetch_details(&self, book_id: &str) -> Result<String> {
        let book_id = book_id.to_string();
        let res = self
            .call_js_fetch("fetchBookDetails", move |func| func.call((book_id,)))
            .await;
        return res;
    }

    pub async fn fetch_chapters_list(&self, book_id: &str, page: i32) -> Result<String> {
        let book_id = book_id.to_string();
        self.call_js_fetch("fetchChaptersList", move |func| func.call((book_id, page)))
            .await
    }

    pub async fn fetch_chapter_content(&self, book_id: &str, chapter_id: &str) -> Result<String> {
        let book_id = book_id.to_string();
        let chapter_id = chapter_id.to_string();
        self.call_js_fetch("fetchChapterContent", move |func| {
            func.call((book_id, chapter_id))
        })
        .await
    }

    pub async fn fetch_search(&self, keyword: &str, genre: Option<&str>) -> Result<String> {
        let _search_config = self
            .config
            .search
            .as_ref()
            .ok_or_else(|| anyhow!("Search capability is not configured for this source"))?;

        let keyword = keyword.to_string();
        let genre_val = genre.unwrap_or("").to_string();

        self.call_js_fetch("fetchSearch", move |func| func.call((keyword, genre_val)))
            .await
    }

    /// A consolidated, generic async runner that handles thread offloading (spawn_blocking).
    async fn call_js_fetch<F>(&self, function_name: &'static str, call_op: F) -> Result<String>
    where
        F: FnOnce(Function) -> Result<String, rquickjs::Error> + Send + 'static,
    {
        // Capture required variables for the blocking thread
        let source_id = self.source_id.clone();
        let script_path = self.config.script_path.clone();
        let client = self.client.clone();
        let handle = tokio::runtime::Handle::current();

        // Move the CPU-heavy QuickJS execution entirely to a blocking worker thread
        tokio::task::spawn_blocking(move || {
            let script = if let Some(ref path) = script_path {
                crate::storage::load_script_content(&source_id, path).map_err(|e| {
                    rquickjs::Error::new_into_js_message("io", "String", e.to_string())
                })?
            } else {
                String::new()
            };

            let rt = Runtime::new()?;
            let ctx = Context::full(&rt)?;

            ctx.with(|ctx| {
                // Register helpers with our pre-built client and handle
                let globals = ctx.globals();

                // Set up fetchUrl native binding
                globals.set(
                    "fetchUrl",
                    rquickjs::Function::new(
                        ctx.clone(),
                        move |url: String| -> Result<String, rquickjs::Error> {
                            let client = client.clone();
                            println!("{}", &url);
                            handle.block_on(async move {
                                let resp = client.get(&url).send().await.map_err(|e| {
                                    rquickjs::Error::new_into_js_message(
                                        "reqwest",
                                        "String",
                                        e.to_string(),
                                    )
                                })?;
                                let text = resp.text().await.map_err(|e| {
                                    rquickjs::Error::new_into_js_message(
                                        "reqwest",
                                        "String",
                                        e.to_string(),
                                    )
                                })?;
                                Ok(text)
                            })
                        },
                    )?,
                )?;

                // Set up sleep native binding to prevent rate limits
                globals.set(
                    "sleep",
                    rquickjs::Function::new(
                        ctx.clone(),
                        |ms: u64| {
                            std::thread::sleep(std::time::Duration::from_millis(ms));
                        },
                    )?,
                )?;

                // Evaluate script and invoke target function
                ctx.eval::<(), _>(script.as_bytes())?;
                let func: Function = globals.get(function_name)?;
                let val: String = call_op(func)?;
                Ok(val)
            })
        })
        .await
        .map_err(|e| anyhow!("JS Execution thread panicked: {}", e))?
    }
}
