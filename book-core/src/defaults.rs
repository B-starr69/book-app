//! Default source configurations.

use crate::{
    models::{
        ActionConfig, DynamicMode, FetchMethod, NativeTarget, ParseMethod, Source, SourceConfig,
        SourceWithConfig,
    },
    BookFormat,
};

/// Get the default NovelFire source configuration.
pub fn novelfire_source() -> SourceWithConfig {
    SourceWithConfig {
        source: Source {
            id: "novelfire".to_string(),
            url: "https://novelfire.net".to_string(),
            cover_url_pattern: "https://novelfire.net/".to_string(),
            name: "NovelFire".to_string(),
            icon_url: None,
            description: Some("Novel reading platform".to_string()),
        },
        config: novelfire_config(),
    }
}

fn novelfire_config() -> SourceConfig {
    SourceConfig {
        default_format: crate::models::SourceType::WebNovel,
        script_path: Some("sources/novelfire/index.js".to_string()),
        home: ActionConfig {
            fetch: FetchMethod::Js,
            parse: ParseMethod::Js,
        },
        details: ActionConfig {
            fetch: FetchMethod::Js,
            parse: ParseMethod::Js,
        },
        chapter: ActionConfig {
            fetch: FetchMethod::Js,
            parse: ParseMethod::Js,
        },
        search: Some(ActionConfig {
            fetch: FetchMethod::Js,
            parse: ParseMethod::Js,
        }),
        genres: vec![],
        chapters_list: ActionConfig {
            fetch: FetchMethod::Js,
            parse: ParseMethod::Js,
        },
    }
}

/// Get all default source configurations
pub fn all_default_sources() -> Vec<SourceWithConfig> {
    vec![novelfire_source()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database, models::ActionEngine, Database};

    #[test]
    fn novelfire_uses_js_strategy() {
        let source = novelfire_source();
    }
}
