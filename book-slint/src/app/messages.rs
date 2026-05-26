use book_core::{Book, HomeSection, SearchResult};

/// Messages sent from background threads to UI.
pub enum Message {
    LibraryLoaded(Vec<Book>),
    DiscoverLoaded { source_id: String, sections: Vec<HomeSection> },
    BookDetailsLoaded(Book),
    ChapterContentLoaded { content: String, book_id: String, chapter_id: String },
    SearchResults(Vec<SearchResult>),
    CoverLoaded { source_id: String, book_id: String, image_data: Vec<u8> },
    ChapterProgress { book_id: String, chapter_id: String, progress: f32 },
    Error(String),
    BookAdded(Book),
    BookRemoved { book_id: String },
    ImportResult(Vec<String>),
}
