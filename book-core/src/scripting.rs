use reqwest::Client;
use rquickjs::{
    async_with,
    prelude::Async,
    AsyncContext, AsyncRuntime, Error as JsError, Function, Value,
};
use scraper::{Html, Selector};

/// Scripting Engine for running Source Extensions natively connecting async Rust to async JS.
pub struct ScriptEngine {
    runtime: AsyncRuntime,
    context: AsyncContext,
    client: Client,
}

impl ScriptEngine {
    /// Create a new asynchronous QuickJS engine specifically tuned for scraping extensions
    pub async fn new() -> Result<Self, JsError> {
        let runtime = AsyncRuntime::new().unwrap();
        let context = AsyncContext::full(&runtime).await.unwrap();

        // Use a shared HTTP client across all scripts for connection pooling
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap();

        let engine = Self {
            runtime,
            context,
            client,
        };
        engine.setup_globals().await?;

        Ok(engine)
    }

    /// Setup the Rust functions that JS scripts can call (fetch, select, etc.)
    async fn setup_globals(&self) -> Result<(), JsError> {
        let client = self.client.clone();

        async_with!(self.context => |ctx| {
            let globals = ctx.globals();

            // fetch_html(url: string) -> Promise<string>
            let fetch_html_client = client.clone();
            let fetch_html = Function::new(ctx.clone(), Async(move |url: String| {
                let client = fetch_html_client.clone();
                async move {
                    let resp = client.get(&url).send().await.map_err(|_| JsError::Exception)?;
                    let text = resp.text().await.map_err(|_| JsError::Exception)?;
                    Ok::<String, JsError>(text)
                }
            }))?;
            globals.set("fetch_html", fetch_html)?;

            // fetch_json(url: string) -> Promise<string> (we'll just return parsed text since JSON requires tied lifetimes, and let JS JSON.parse it)
            let fetch_json_client = client.clone();
            let fetch_json = Function::new(ctx.clone(), Async(move |url: String| {
                let client = fetch_json_client.clone();
                async move {
                    let resp = client.get(&url).send().await.map_err(|_| JsError::Exception)?;
                    let text = resp.text().await.map_err(|_| JsError::Exception)?;
                    Ok::<String, JsError>(text)
                }
            }))?;
            globals.set("fetch_json", fetch_json)?;

            // select_text(html: string, selector: string) -> string
            let select_text = Function::new(
                ctx.clone(),
                |html: String, selector: String| -> Result<Option<String>, JsError> {
                    let document = Html::parse_document(&html);
                    if let Ok(sel) = Selector::parse(&selector) {
                        if let Some(element) = document.select(&sel).next() {
                            let text = element.text().collect::<Vec<_>>().join(" ");
                            return Ok(Some(text.trim().to_string()));
                        }
                    }
                    Ok(None)
                },
            )?;
            globals.set("select_text", select_text)?;

            // select_attr(html: string, selector: string, attr: string) -> string
            let select_attr = Function::new(
                ctx.clone(),
                |html: String, selector: String, attr: String| -> Result<Option<String>, JsError> {
                    let document = Html::parse_document(&html);
                    if let Ok(sel) = Selector::parse(&selector) {
                        if let Some(element) = document.select(&sel).next() {
                            return Ok(element.value().attr(&attr).map(|a| a.to_string()));
                        }
                    }
                    Ok(None)
                },
            )?;
            globals.set("select_attr", select_attr)?;

            // select_html(html: string, selector: string) -> string
             let select_html = Function::new(
                ctx.clone(),
                |html: String, selector: String| -> Result<Option<String>, JsError> {
                    let document = Html::parse_document(&html);
                    if let Ok(sel) = Selector::parse(&selector) {
                        if let Some(element) = document.select(&sel).next() {
                            return Ok(Some(element.inner_html()));
                        }
                    }
                    Ok(None)
                },
            )?;
            globals.set("select_html", select_html)?;

            Ok(())
        })
        .await
    }

    /// Load a script string and evaluate it
    pub async fn load_script(&self, script: &str) -> Result<(), JsError> {
        async_with!(self.context => |ctx| {
            // Evaluate the JS script. This executes top-level code and exposes `source`
            let _val: Value = ctx.eval(script)?;
            Ok(())
        })
        .await
    }
}
