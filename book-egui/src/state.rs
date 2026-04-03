use book_core::{Book, HomeSection, SearchResult, SourceWithConfig};

/// Streaming load state for progressive UI
#[derive(Debug, Clone, Default, PartialEq)]
pub enum StreamingState {
    #[default]
    Idle,
    /// Currently loading, with count of items loaded so far
    Loading { items_loaded: usize },
    /// Finished loading
    Done,
    /// Error occurred during loading
    Error(String),
}

impl StreamingState {
    pub fn is_loading(&self) -> bool {
        matches!(self, StreamingState::Loading { .. })
    }

    pub fn is_done(&self) -> bool {
        matches!(self, StreamingState::Done)
    }
}

/// Shared application state that both UI and Logic threads can access
#[derive(Default)]
pub struct AppState {
    // Data
    pub library_books: Vec<Book>,
    pub discover_sections: Vec<HomeSection>,
    pub current_book: Option<Book>,
    pub chapter_content: String,

    // Sources
    pub sources: Vec<SourceWithConfig>,
    pub selected_source: Option<SourceWithConfig>,

    // Loading states
    pub is_loading: bool,
    pub is_searching: bool,
    pub is_global_searching: bool,

    // Streaming states for progressive UI
    pub discover_streaming: StreamingState,
    pub global_search_streaming: StreamingState,
    pub book_details_streaming: StreamingState,

    // Track if we need to save the book after streaming completes
    pub needs_save_after_streaming: bool,

    // Error
    pub error_message: Option<String>,

    // Search
    pub search_results: Vec<SearchResult>,
    pub global_search_results: Vec<SearchResult>,
}

/// Commands sent from UI thread to Logic thread
#[derive(Debug, Clone)]
pub enum Command {
    LoadDiscover,
    LoadBookDetails { book_id: String },
    LoadChapter { book_id: String, chapter_id: String },
    RefreshCurrentBook,
    RefreshLibraryBooks,
    Search { query: String },
    GlobalSearch { query: String },
    AddToLibrary { book: Book },
    RemoveFromLibrary { book_id: String, source_id: String },
    ChangeSource { source: SourceWithConfig },
    CacheCover { source_id: String, book_id: String, cover_url: String },
    // Source management
    SaveSource { source: SourceWithConfig },
    DeleteSource { source_id: String },
    // Reading progress
    UpdateChapterProgress { book_id: String, chapter_id: String, progress: f32 },
    // Cache book with chapters after streaming
    SaveBookToCache { book: Box<Book> },
}

/// Application views/screens
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Library,
    Discover,
    Search,
    Settings,
    BookDetails(String),    // book_id
    Reader(String, String), // book_id, chapter_id
}
