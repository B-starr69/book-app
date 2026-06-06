pub mod api;
pub mod configurable_parser;
pub mod database;
pub mod defaults;
pub mod getter;
pub mod importer;
pub mod native_parser;
pub mod models;
pub mod platform;
pub mod parser_utils;

// Re-export commonly used types at crate root
pub use configurable_parser::ConfigurableParser;
pub use database::Database;
pub use native_parser::NativeParser;
pub use models::{
    Book, Chapter, ChapterSelectors, DetailsSelectors, GenreInfo,
    HomeSection, HomeSelectors, LayoutMapping, ParsedBookDetails, ParsedChapter, ParsedChapterInfo,
    SearchResult, SectionLayout, Source, SourceConfig, SourceWithConfig,
};

pub use importer::{import_from_github, check_for_updates};
