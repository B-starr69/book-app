pub mod api;
pub mod configurable_parser;
pub mod database;
pub mod defaults;
pub mod fetcher;
pub mod importer;
pub mod models;
pub mod platform;
pub mod storage;

pub use configurable_parser::ConfigurableParser;
pub use database::{Database, DatabaseMode, TursoConfig};
pub use importer::{check_for_updates, import_from_github};
pub use models::{
    BaseBook, Book, BookFormat, Chapter, ChapterContent, GenreInfo, HomeSection, Novel,
    ParsedBookDetails, ParsedChapter, ParsedChapterInfo, SearchResult, SectionLayout, Source,
    SourceConfig, SourceWithConfig, WebNovel,
};
pub use storage::{import_local_file, load_webnovel_chapter_html, save_webnovel_chapter_html};
