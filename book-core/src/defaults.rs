//! Default source configurations.

use crate::models::{
    ActionConfig, FetchMethod, ParseMethod, Source, SourceConfig,
    SourceWithConfig,
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

    #[test]
    fn novelfire_uses_js_strategy() {
        let _source = novelfire_source();
    }

    #[test]
    fn test_parse_chapter_content() {
        let source = novelfire_source();
        let parser = crate::configurable_parser::ConfigurableParser::new(source.source.id, source.config);
        
        let html_content = std::fs::read_to_string("../sources/novelfire/chapter_example.html")
            .unwrap_or_else(|_| std::fs::read_to_string("sources/novelfire/chapter_example.html").unwrap());
            
        let parsed = parser.parse_chapter_content(&html_content).unwrap();
        
        assert_eq!(
            parsed.title,
            "Chapter 1: Countdown to Extending Life - I Made the Devilish Screenwriter Cry!"
        );
        assert!(parsed.content.contains("Palace Intrigue"));
    }
}
