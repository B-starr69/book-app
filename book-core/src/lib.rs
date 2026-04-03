pub mod api;
pub mod configurable_parser;
pub mod database;
pub mod defaults;
pub mod getter;
pub mod models;
pub mod platform;

// Re-export commonly used types at crate root
pub use configurable_parser::ConfigurableParser;
pub use database::Database;
pub use models::{
    Book, BookPreview, Chapter, ChapterSelectors, DbBook, DbChapter, DetailsSelectors, HomeSection,
    HomeSelectors, LayoutMapping, ParsedBookDetails, ParsedChapter, ParsedChapterInfo,
    SearchConfig, SearchResult, SearchResultMapping, SectionLayout, Source, SourceConfig,
    SourceWithConfig,
};
