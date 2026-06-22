//! Default source configurations.

use crate::models::{
    ActionConfig, DynamicMode, FetchMethod, JsExecutionConfig, NativeTarget, Source, SourceConfig,
    SourceWithConfig, Strategy,
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
        script_path: Some("sources/novelfire/index.js".to_string()),
        home: ActionConfig {
            fetch: FetchMethod::Native {
                target: NativeTarget::Static {
                    url: "https://novelfire.net/home".to_string(),
                },
            },
            parse: Strategy::Js(JsExecutionConfig {
                js_function: Some("parseHome".to_string()),
                script: None,
            }),
        },
        details: ActionConfig {
            fetch: FetchMethod::default(),
            parse: Strategy::Js(JsExecutionConfig {
                js_function: Some("parseBookDetails".to_string()),
                script: None,
            }),
        },
        chapter: ActionConfig {
            fetch: FetchMethod::default(),
            parse: Strategy::Js(JsExecutionConfig {
                js_function: Some("parseChapterContent".to_string()),
                script: None,
            }),
        },
        search: Some(ActionConfig {
            fetch: FetchMethod::Native {
                target: NativeTarget::Dynamic {
                    url_pattern:
                        "https://novelfire.net/ajax/searchLive?keyword={keyword}&type=title"
                            .to_string(),
                    mode: DynamicMode::Single,
                },
            },
            parse: Strategy::Js(JsExecutionConfig {
                js_function: Some("parseSearch".to_string()),
                script: None,
            }),
        }),
        genres: vec![],
        chapters_list: ActionConfig {
            fetch: FetchMethod::default(),
            parse: Strategy::Js(JsExecutionConfig {
                js_function: Some("parseChapters".to_string()),
                script: None,
            }),
        },
        ..Default::default()
    }
}

/// Get all default source configurations
pub fn all_default_sources() -> Vec<SourceWithConfig> {
    vec![novelfire_source()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActionEngine;

    #[test]
    fn novelfire_uses_js_strategy() {
        let source = novelfire_source();
        assert_eq!(source.source.id, "novelfire");
        assert!(matches!(
            source.config.home.effective_engine(),
            ActionEngine::Js
        ));
        assert!(matches!(
            source.config.details.effective_engine(),
            ActionEngine::Js
        ));
        assert!(matches!(
            source.config.chapter.effective_engine(),
            ActionEngine::Js
        ));
    }
}
