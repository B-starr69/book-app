use crate::cover_cache::get_cached_cover_url;
use crate::logic::run_logic_thread;
use crate::state::{AppState, Command, StreamingState, View};
use book_core::{defaults, Book, Database, HomeSection, SearchResult, SourceWithConfig};
use eframe::egui;
use egui::Context;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};

/// Main application struct - runs on UI thread
pub struct BookApp {
    // Shared state (read by UI, written by Logic thread)
    pub state: Arc<RwLock<AppState>>,

    // Command channel to logic thread
    pub cmd_tx: Sender<Command>,

    // UI-only state (not shared)
    pub current_view: View,
    pub dark_mode: bool,
    pub reader_font_size: f32,
    pub search_query: String,
    pub global_search_query: String,
    pub pending_cover_downloads: HashSet<String>,
    pub summary_expanded: bool,

    // Source editing state
    pub editing_source: Option<SourceWithConfig>,
    pub source_json_input: String,
    pub source_error: Option<String>,

    // Reader scroll tracking
    pub reader_scroll_offset: f32,
    pub reader_needs_scroll_restore: bool,
    pub reader_target_progress: f32,
    pub reader_last_content_height: f32,

    // Auto-save tracking
    pub last_auto_save: f64,
    pub last_saved_progress_value: f32,

    // Debounce rapid chapter navigation
    pub last_chapter_switch_time: f64,
    pub currently_loading_chapter_id: Option<String>,

    // UI Toggle State
    pub show_reader_controls: bool,
    pub show_search_bar: bool,
}

impl BookApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Ensure covers directory exists
        let _ = std::fs::create_dir_all("covers");

        // Setup custom fonts if needed
        crate::setup_custom_fonts(&cc.egui_ctx);

        // Enable image loading from URLs
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Initialize database
        let database = Database::new().ok();

        // Load sources with configs from database
        let mut sources = if let Some(ref db) = database {
            db.get_sources_with_config().unwrap_or_default()
        } else {
            vec![]
        };

        // Add default sources if none exist
        if sources.is_empty() {
            let default_source = defaults::novelfire_source();
            if let Some(ref db) = database {
                let _ = db.save_source_with_config(&default_source);
            }
            sources.push(default_source);
        }

        // Load library books from database
        let library_books = if let Some(ref db) = database {
            db.get_library_books().unwrap_or_default()
        } else {
            vec![]
        };

        let selected_source = sources.first().cloned();

        // Create shared state
        let state = Arc::new(RwLock::new(AppState {
            library_books,
            discover_sections: vec![],
            current_book: None,
            chapter_content: String::new(),
            sources: sources.clone(),
            selected_source: selected_source.clone(),
            is_loading: false,
            is_searching: false,
            is_global_searching: false,
            discover_streaming: StreamingState::Idle,
            global_search_streaming: StreamingState::Idle,
            book_details_streaming: StreamingState::Idle,
            needs_save_after_streaming: false,
            error_message: None,
            search_results: vec![],
            global_search_results: vec![],
        }));

        // Create command channel (UI → Logic)
        let (cmd_tx, cmd_rx) = channel::<Command>();

        // Clone for logic thread
        let state_clone = Arc::clone(&state);
        let ctx_clone = cc.egui_ctx.clone();
        let db_path = database.as_ref().map(|_| "books.db".to_string());

        // Start logic thread
        std::thread::spawn(move || {
            run_logic_thread(state_clone, cmd_rx, ctx_clone, db_path);
        });

        Self {
            state,
            cmd_tx,
            current_view: View::Library,
            dark_mode: true,
            reader_font_size: 18.0,
            search_query: String::new(),
            global_search_query: String::new(),
            pending_cover_downloads: HashSet::new(),
            summary_expanded: false,
            editing_source: None,
            source_json_input: String::new(),
            source_error: None,
            reader_scroll_offset: 0.0,
            reader_needs_scroll_restore: false,
            reader_target_progress: 0.0,
            reader_last_content_height: 0.0,
            last_auto_save: 0.0,
            last_saved_progress_value: -1.0, // Force save on first update
            last_chapter_switch_time: 0.0,
            currently_loading_chapter_id: None,
            show_reader_controls: true,
            show_search_bar: false,
        }
    }

    // ============================================================================
    // COMMAND SENDERS - Send commands to logic thread
    // ============================================================================

    pub fn load_discover(&mut self, _ctx: &Context) {
        let _ = self.cmd_tx.send(Command::LoadDiscover);
    }

    pub fn load_book_details(&mut self, book_id: String, _ctx: &Context) {
        let _ = self.cmd_tx.send(Command::LoadBookDetails { book_id });
    }

    pub fn load_chapter(&mut self, book_id: String, chapter_id: String, ctx: &Context) {
        // Debounce rapid chapter switches (100ms minimum between chapters)
        let current_time = ctx.input(|i| i.time);
        if current_time - self.last_chapter_switch_time < 0.1 {
            return; // Ignore rapid clicks
        }

        self.last_chapter_switch_time = current_time;
        self.currently_loading_chapter_id = Some(chapter_id.clone());

        // Check for existing progress to restore scroll position
        let target_progress = {
            let state = self.state.read().unwrap();
            let progress = state.current_book.as_ref()
                .and_then(|book| book.chapters.iter().find(|c| c.id == chapter_id))
                .map(|ch| ch.progress)
                .unwrap_or(0.0);

            // Also check library books
            if progress == 0.0 {
                state.library_books.iter()
                    .find(|b| b.id == book_id)
                    .and_then(|book| book.chapters.iter().find(|c| c.id == chapter_id))
                    .map(|ch| ch.progress)
                    .unwrap_or(0.0)
            } else {
                progress
            }
        };

        // Reset scroll state for new chapter
        self.reader_scroll_offset = 0.0;
        self.reader_target_progress = target_progress;
        self.reader_needs_scroll_restore = target_progress > 0.0 && target_progress < 1.0;
        self.last_auto_save = 0.0;
        self.last_saved_progress_value = target_progress;
        self.show_reader_controls = true; // Always show controls when opening

        let _ = self.cmd_tx.send(Command::LoadChapter { book_id, chapter_id });
    }

    pub fn save_chapter_progress(&mut self, book_id: &str, chapter_id: &str, progress: f32) {
        let _ = self.cmd_tx.send(Command::UpdateChapterProgress {
            book_id: book_id.to_string(),
            chapter_id: chapter_id.to_string(),
            progress,
        });
    }

    pub fn refresh_current_book(&mut self, _ctx: &Context) {
        let _ = self.cmd_tx.send(Command::RefreshCurrentBook);
    }

    pub fn refresh_library_books(&mut self, _ctx: &Context) {
        let _ = self.cmd_tx.send(Command::RefreshLibraryBooks);
    }

    pub fn add_to_library(&mut self, book: &Book) {
        let _ = self.cmd_tx.send(Command::AddToLibrary { book: book.clone() });
    }

    pub fn remove_from_library(&mut self, book_id: &str, source_id: &str) {
        let _ = self.cmd_tx.send(Command::RemoveFromLibrary {
            book_id: book_id.to_string(),
            source_id: source_id.to_string(),
        });
    }

    pub fn perform_search(&mut self, _ctx: &Context) {
        if !self.search_query.trim().is_empty() {
            let _ = self.cmd_tx.send(Command::Search {
                query: self.search_query.clone(),
            });
        }
    }

    pub fn perform_global_search(&mut self, _ctx: &Context) {
        if !self.global_search_query.trim().is_empty() {
            let _ = self.cmd_tx.send(Command::GlobalSearch {
                query: self.global_search_query.clone(),
            });
        }
    }

    pub fn change_source(&mut self, source: SourceWithConfig) {
        let _ = self.cmd_tx.send(Command::ChangeSource { source });
    }

    pub fn save_source(&mut self, source: SourceWithConfig) {
        let _ = self.cmd_tx.send(Command::SaveSource { source });
    }

    pub fn delete_source(&mut self, source_id: String) {
        let _ = self.cmd_tx.send(Command::DeleteSource { source_id });
    }

    pub fn get_sources(&self) -> Vec<SourceWithConfig> {
        self.state.read().map(|s| s.sources.clone()).unwrap_or_default()
    }
    // ============================================================================
    // STATE ACCESSORS - Read from shared state
    // ============================================================================

    pub fn is_loading(&self) -> bool {
        self.state.read().map(|s| s.is_loading).unwrap_or(false)
    }

    pub fn is_searching(&self) -> bool {
        self.state.read().map(|s| s.is_searching).unwrap_or(false)
    }

    pub fn is_global_searching(&self) -> bool {
        self.state
            .read()
            .map(|s| s.is_global_searching)
            .unwrap_or(false)
    }

    pub fn get_discover_streaming_state(&self) -> StreamingState {
        self.state
            .read()
            .map(|s| s.discover_streaming.clone())
            .unwrap_or_default()
    }

    pub fn get_global_search_streaming_state(&self) -> StreamingState {
        self.state
            .read()
            .map(|s| s.global_search_streaming.clone())
            .unwrap_or_default()
    }

    pub fn get_book_details_streaming_state(&self) -> StreamingState {
        self.state
            .read()
            .map(|s| s.book_details_streaming.clone())
            .unwrap_or_default()
    }

    pub fn get_error_message(&self) -> Option<String> {
        self.state.read().ok().and_then(|s| s.error_message.clone())
    }

    pub fn get_library_books(&self) -> Vec<Book> {
        self.state
            .read()
            .map(|s| s.library_books.clone())
            .unwrap_or_default()
    }

    pub fn get_discover_sections(&self) -> Vec<HomeSection> {
        self.state
            .read()
            .map(|s| s.discover_sections.clone())
            .unwrap_or_default()
    }

    pub fn get_current_book(&self) -> Option<Book> {
        self.state.read().ok().and_then(|s| s.current_book.clone())
    }

    pub fn get_chapter_content(&self) -> String {
        self.state
            .read()
            .map(|s| s.chapter_content.clone())
            .unwrap_or_default()
    }

    pub fn get_selected_source(&self) -> Option<SourceWithConfig> {
        self.state
            .read()
            .ok()
            .and_then(|s| s.selected_source.clone())
    }

    pub fn get_search_results(&self) -> Vec<SearchResult> {
        self.state
            .read()
            .map(|s| s.search_results.clone())
            .unwrap_or_default()
    }

    pub fn get_global_search_results(&self) -> Vec<SearchResult> {
        self.state
            .read()
            .map(|s| s.global_search_results.clone())
            .unwrap_or_default()
    }

    // ============================================================================
    // COVER CACHE HELPERS - Delegate to cover_cache module
    // ============================================================================

    pub fn get_cached_cover_url(source_id: &str, book_id: &str) -> Option<String> {
        get_cached_cover_url(source_id, book_id)
    }
}
