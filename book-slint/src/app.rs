// ============================================================================
// App State & Logic
// ============================================================================

use crate::{App, BookData, ChapterData, SectionData, SearchResultData, ViewState};
use book_core::{Book, Chapter, Database, HomeSection, SearchResult, Source};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Image, Rgba8Pixel, SharedPixelBuffer};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};

/// Messages sent from background threads to UI
pub enum Message {
    LibraryLoaded(Vec<Book>),
    DiscoverLoaded(Vec<HomeSection>),
    DiscoverSectionAdded(HomeSection),
    BookDetailsLoaded(Book),
    ChapterContentLoaded { content: String, book_id: String, chapter_id: String },
    SearchResults(Vec<SearchResult>),
    CoverLoaded { book_id: String, image_data: Vec<u8> },
    ChapterProgress { book_id: String, chapter_id: String, progress: f32 },
    Error(String),
    BookAdded(Book),
    BookRemoved { book_id: String },
    ImportResult(Vec<String>),
}

/// Cover cache helper functions
mod cover_cache {
    use std::path::PathBuf;

    pub fn get_cover_cache_path(source_id: &str, book_id: &str) -> PathBuf {
        let covers_dir = PathBuf::from("covers");
        let base = format!("{}_{}", source_id, book_id);

        for ext in &["jpg", "jpeg", "png", "webp", "gif"] {
            let path = covers_dir.join(format!("{}.{}", base, ext));
            if path.exists() {
                return path;
            }
        }
        covers_dir.join(base)
    }

    pub fn get_cached_cover_path(source_id: &str, book_id: &str) -> Option<PathBuf> {
        let path = get_cover_cache_path(source_id, book_id);
        if path.exists() && path.extension().is_some() {
            Some(path)
        } else {
            None
        }
    }

    pub fn cache_cover_sync(source_id: &str, book_id: &str, cover_url: &str) {
        if cover_url.is_empty() {
            return;
        }

        let existing = get_cover_cache_path(source_id, book_id);
        if existing.exists() && existing.extension().is_some() {
            return;
        }

        let _ = std::fs::create_dir_all("covers");

        let client = reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        match client.get(cover_url).send() {
            Ok(response) => {
                if !response.status().is_success() {
                    return;
                }

                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                let ext = if content_type.contains("png") {
                    "png"
                } else if content_type.contains("webp") {
                    "webp"
                } else if content_type.contains("gif") {
                    "gif"
                } else if content_type.contains("jpeg") || content_type.contains("jpg") {
                    "jpg"
                } else if cover_url.contains(".png") {
                    "png"
                } else if cover_url.contains(".webp") {
                    "webp"
                } else {
                    "jpg"
                };

                match response.bytes() {
                    Ok(bytes) => {
                        if bytes.len() < 8 {
                            return;
                        }

                        let is_image = bytes.starts_with(&[0xFF, 0xD8, 0xFF])
                            || bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47])
                            || bytes.starts_with(b"RIFF")
                            || bytes.starts_with(b"GIF");

                        if !is_image {
                            return;
                        }

                        let covers_dir = PathBuf::from("covers");
                        let path = covers_dir.join(format!("{}_{}.{}", source_id, book_id, ext));
                        let _ = std::fs::write(&path, &bytes);
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    }
}

/// Main application wrapper
pub struct BookApp {
    ui: App,
}

impl BookApp {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = App::new()?;
        let (msg_tx, msg_rx) = channel::<Message>();

        // Initialize database
        let database = Arc::new(Database::new().ok());

        // API functions are called directly from book_core::api

        // Load sources
        let mut sources = if let Some(ref db) = *database {
            db.get_sources().unwrap_or_default()
        } else {
            vec![]
        };

        if sources.is_empty() {
            let default_source = Source {
                id: "novelfire".to_string(),
                url: "https://novelfire.net".to_string(),
                name: "NovelFire".to_string(),
                discover_url: "https://novelfire.net/home".to_string(),
                books_url: "https://novelfire.net/book/".to_string(),
                icon_url: None,
                description: None,
            };
            if let Some(ref db) = *database {
                let _ = db.save_source(&default_source);
            }
            sources.push(default_source);
        }

        let sources = Arc::new(RwLock::new(sources));

        // Load initial library books and cache their covers
        if let Some(ref db) = *database {
            if let Ok(books) = db.get_library_books() {
                let model = Self::books_to_model(&books);
                ui.set_library_books(model);

                // Trigger cover loading for library books
                let tx = msg_tx.clone();
                for book in books.iter() {
                    Self::load_cover_async(&tx, &book.source_id, &book.id, &book.cover_url);
                }
            }
        }

        // Current book state for progress tracking
        let current_book_state = Arc::new(RwLock::new(None::<Book>));

        // Setup callbacks
        Self::setup_callbacks(&ui, msg_tx.clone(), Arc::clone(&sources), Arc::clone(&database), Arc::clone(&current_book_state));

        // Setup timer to poll for async messages
        let ui_weak = ui.as_weak();
        let timer = slint::Timer::default();
        let current_book_for_timer = Arc::clone(&current_book_state);

        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                while let Ok(msg) = msg_rx.try_recv() {
                    if let Some(ui) = ui_weak.upgrade() {
                        Self::handle_message(&ui, msg, &current_book_for_timer);
                    }
                }
            },
        );

        std::mem::forget(timer);

        Ok(Self { ui })
    }

    fn load_cover_async(tx: &Sender<Message>, source_id: &str, book_id: &str, cover_url: &str) {
        let source_id = source_id.to_string();
        let book_id = book_id.to_string();
        let cover_url = cover_url.to_string();
        let tx = tx.clone();

        std::thread::spawn(move || {
            // First check if cached
            if let Some(path) = cover_cache::get_cached_cover_path(&source_id, &book_id) {
                if let Ok(data) = std::fs::read(&path) {
                    let _ = tx.send(Message::CoverLoaded { book_id, image_data: data });
                    return;
                }
            }

            // Otherwise download and cache
            if !cover_url.is_empty() {
                cover_cache::cache_cover_sync(&source_id, &book_id, &cover_url);

                // Now load the cached image
                if let Some(path) = cover_cache::get_cached_cover_path(&source_id, &book_id) {
                    if let Ok(data) = std::fs::read(&path) {
                        let _ = tx.send(Message::CoverLoaded { book_id, image_data: data });
                    }
                }
            }
        });
    }

    fn bytes_to_image(data: &[u8]) -> Option<Image> {
        let img = image::load_from_memory(data).ok()?;
        let rgba = img.to_rgba8();
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
        );
        Some(Image::from_rgba8(buffer))
    }

    fn setup_callbacks(
        ui: &App,
        msg_tx: Sender<Message>,
        sources: Arc<RwLock<Vec<Source>>>,
        _database: Arc<Option<Database>>,
        current_book_state: Arc<RwLock<Option<Book>>>,
    ) {
        // Navigation callback
        let ui_weak = ui.as_weak();
        ui.on_navigate(move |view| {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_current_view(view);
            }
        });

        // Load library callback
        let msg_tx_lib = msg_tx.clone();
        ui.on_load_library(move || {
            let tx = msg_tx_lib.clone();
            std::thread::spawn(move || {
                if let Ok(db) = Database::new() {
                    let books = db.get_library_books().unwrap_or_default();
                    let _ = tx.send(Message::LibraryLoaded(books));
                }
            });
        });

        // Load discover callback
        let msg_tx_discover = msg_tx.clone();
        let sources_discover = Arc::clone(&sources);
        ui.on_load_discover(move || {
            let tx = msg_tx_discover.clone();
            let sources = sources_discover.read().unwrap().clone();
            if let Some(source) = sources.first().cloned() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        // TODO: Implement streaming in API, falling back to dummy for now
                        let (section_tx, section_rx) = std::sync::mpsc::channel::<HomeSection>();
                        let tx_clone = tx.clone();

                        std::thread::spawn(move || {
                            while let Ok(section) = section_rx.recv() {
                                let _ = tx_clone.send(Message::DiscoverSectionAdded(section));
                            }
                        });

                        // Fake discover sections for now to prevent compile errors
                        let fake_section = HomeSection {
                            title: "Discover".to_string(),
                            layout: book_core::models::SectionLayout::Grid,
                            books: vec![]
                        };
                        let _ = section_tx.send(fake_section);
                    });
                });
            }
        });

        // Manage sources callback - show import dialog
        let ui_manage = ui.as_weak();
        ui.on_manage_sources(move || {
            if let Some(ui) = ui_manage.upgrade() {
                ui.set_show_import_dialog(true);
            }
        });

        // Close import dialog
        let ui_close = ui.as_weak();
        ui.on_close_import_dialog(move || {
            if let Some(ui) = ui_close.upgrade() {
                ui.set_show_import_dialog(false);
                ui.set_import_repo_url(SharedString::from(""));
            }
        });

        // Import from GitHub callback
        let msg_tx_import = msg_tx.clone();
        let sources_import = Arc::clone(&sources);
        ui.on_import_github(move |repo_url| {
            let tx = msg_tx_import.clone();
            let sources = Arc::clone(&sources_import);
            std::thread::spawn(move || {
                // Create DB and call async importer
                if let Ok(db) = Database::new() {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let repo = repo_url.to_string();
                    match rt.block_on(async { book_core::import_from_github(&repo, &db).await }) {
                        Ok(imported) => {
                            // Reload sources from DB
                            if let Ok(new_sources) = db.get_sources() {
                                let mut s = sources.write().unwrap();
                                *s = new_sources;
                            }
                            let _ = tx.send(Message::ImportResult(imported));
                        }
                        Err(e) => {
                            let _ = tx.send(Message::Error(format!("Import failed: {}", e)));
                        }
                    }
                } else {
                    let _ = tx.send(Message::Error("Database not available".to_string()));
                }
            });
        });

        // Open book callback
        let msg_tx_book = msg_tx.clone();
        let sources_book = Arc::clone(&sources);
        ui.on_open_book(move |book_id| {
            let tx = msg_tx_book.clone();
            let sources = sources_book.read().unwrap().clone();
            let book_id = book_id.to_string();
                        if let Some(source) = sources.first().cloned() {
                std::thread::spawn(move || {
                    // Check cache first
                    if let Ok(db) = Database::new() {
                        if let Ok(Some(cached_book)) = db.get_full_book(&book_id, &source.id) {
                            let _ = tx.send(Message::BookDetailsLoaded(cached_book));
                            return;
                        }
                    }

                    // Fetch from network
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let source_cfg = book_core::models::SourceWithConfig {
                            id: source.id.clone(),
                            url: source.url.clone(),
                            name: source.name.clone(),
                            discover_url: source.discover_url.clone(),
                            books_url: source.books_url.clone(),
                            icon_url: None,
                            description: None,
                            config: book_core::models::SourceConfig::default(),
                        };
                        if let Some(details) = book_core::api::get_book_details(&source_cfg, book_id.clone()).await {
                            let book = Book {
                                id: book_id.clone(),
                                source_id: source.id.clone(),
                                title: details.title,
                                author: details.author,
                                cover_url: details.cover_url.clone(),
                                rating: details.rating,
                                status: details.status,
                                chapters_count: details.chapters_count,
                                genres: details.genres,
                                summary: details.summary,
                                in_library: false,
                                chapters: details.chapters.into_iter().map(|c| Chapter {
                                    id: c.id,
                                    title: c.title,
                                    date: c.date,
                                    progress: 0.0,
                                    last_read: 0,
                                }).collect(),
                            };

                            // Cache the book
                            if let Ok(db) = Database::new() {
                                let _ = db.save_full_book(&book);
                            }

                            // Load cover
                            let tx2 = tx.clone();
                            let source_id = source.id.clone();
                            let bid = book_id.clone();
                            let curl = details.cover_url.clone();
                            std::thread::spawn(move || {
                                cover_cache::cache_cover_sync(&source_id, &bid, &curl);
                                if let Some(path) = cover_cache::get_cached_cover_path(&source_id, &bid) {
                                    if let Ok(data) = std::fs::read(&path) {
                                        let _ = tx2.send(Message::CoverLoaded { book_id: bid, image_data: data });
                                    }
                                }
                            });

                            let _ = tx.send(Message::BookDetailsLoaded(book));
                        }
                    });
                });
            }
        });

        // Read chapter callback
        let msg_tx_chapter = msg_tx.clone();
        let sources_chapter = Arc::clone(&sources);
        ui.on_read_chapter(move |book_id, chapter_id| {
            let tx = msg_tx_chapter.clone();
            let sources = sources_chapter.read().unwrap().clone();
            let book_id = book_id.to_string();
            let chapter_id = chapter_id.to_string();
                        if let Some(source) = sources.first().cloned() {
                std::thread::spawn(move || {
                    // Fetch from network
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let source_cfg = book_core::models::SourceWithConfig {
                            id: source.id.clone(),
                            url: source.url.clone(),
                            name: source.name.clone(),
                            discover_url: source.discover_url.clone(),
                            books_url: source.books_url.clone(),
                            icon_url: None,
                            description: None,
                            config: book_core::models::SourceConfig::default(),
                        };
                        if let Some(chapter) = book_core::api::get_chapter_content(&source_cfg, book_id.clone(), chapter_id.clone()).await {
                            let _ = tx.send(Message::ChapterContentLoaded {
                                content: chapter.content,
                                book_id,
                                chapter_id
                            });
                        }
                    });
                });
            }
        });

        // Update chapter progress callback
        let msg_tx_progress = msg_tx.clone();
        ui.on_update_chapter_progress(move |book_id, chapter_id, progress| {
            let _ = msg_tx_progress.send(Message::ChapterProgress {
                book_id: book_id.to_string(),
                chapter_id: chapter_id.to_string(),
                progress,
            });
        });

        // Add to library callback
        let msg_tx_add = msg_tx.clone();
        let current_book_add = Arc::clone(&current_book_state);
        ui.on_add_to_library(move || {
            let tx = msg_tx_add.clone();
            let current = Arc::clone(&current_book_add);

            std::thread::spawn(move || {
                if let Some(mut book) = current.read().unwrap().clone() {
                    book.in_library = true;
                    if let Ok(db) = Database::new() {
                        let _ = db.save_full_book(&book);
                    }
                    let _ = tx.send(Message::BookAdded(book));
                }
            });
        });

        // Remove from library callback
        let msg_tx_remove = msg_tx.clone();
        let current_book_remove = Arc::clone(&current_book_state);
        let sources_remove = Arc::clone(&sources);
        ui.on_remove_from_library(move || {
            let tx = msg_tx_remove.clone();
            let current = Arc::clone(&current_book_remove);
            let sources = sources_remove.read().unwrap().clone();

            std::thread::spawn(move || {
                if let Some(book) = current.read().unwrap().clone() {
                    if let Ok(db) = Database::new() {
                        if let Some(source) = sources.first() {
                            let _ = db.remove_from_library(&book.id, &source.id);
                        }
                    }
                    let _ = tx.send(Message::BookRemoved { book_id: book.id });
                }
            });
        });

        // Search callback
        let sources_search = Arc::clone(&sources);
        ui.on_search(move |query| {
            let tx = msg_tx.clone();
            let sources = sources_search.read().unwrap().clone();
            let query = query.to_string();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Some(source) = sources.first() {
                        let source_cfg = book_core::models::SourceWithConfig {
                            id: source.id.clone(),
                            url: source.url.clone(),
                            name: source.name.clone(),
                            discover_url: source.discover_url.clone(),
                            books_url: source.books_url.clone(),
                            icon_url: None,
                            description: None,
                            config: book_core::models::SourceConfig::default(),
                        };
                        if let Some(results) = book_core::api::search_books(&source_cfg, &query).await {
                            let _ = tx.send(Message::SearchResults(results));
                        }
                    }
                });
            });
        });
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }

    fn handle_message(
        ui: &App,
        msg: Message,
        current_book_state: &Arc<RwLock<Option<Book>>>,
    ) {
        match msg {
            Message::LibraryLoaded(books) => {
                ui.set_library_books(Self::books_to_model(&books));
                ui.set_is_loading(false);
            }
            Message::DiscoverLoaded(sections) => {
                ui.set_discover_sections(Self::sections_to_model(&sections));
                ui.set_is_loading(false);
            }
            Message::DiscoverSectionAdded(section) => {
                // Append section to existing model
                let current = ui.get_discover_sections();
                let section_data = Self::section_to_slint(&section);

                // Create new model with appended section
                let items: Vec<SectionData> = (0..current.row_count())
                    .filter_map(|i| current.row_data(i))
                    .chain(std::iter::once(section_data))
                    .collect();

                ui.set_discover_sections(ModelRc::new(VecModel::from(items)));
                ui.set_is_loading(false);
            }
            Message::BookDetailsLoaded(book) => {
                // Store current book for progress tracking
                *current_book_state.write().unwrap() = Some(book.clone());

                ui.set_current_book(Self::book_to_slint(&book));
                ui.set_book_chapters(Self::chapters_to_model(&book.chapters));
                ui.set_current_view(ViewState::BookDetails);
                ui.set_is_loading(false);
            }
            Message::ChapterContentLoaded { content, book_id, chapter_id } => {
                ui.set_chapter_content(SharedString::from(&content));
                ui.set_current_chapter_id(SharedString::from(&chapter_id));
                ui.set_current_book_id(SharedString::from(&book_id));
                ui.set_current_view(ViewState::Reader);
                ui.set_is_loading(false);
            }
            Message::SearchResults(results) => {
                ui.set_search_results(Self::search_results_to_model(&results));
                ui.set_is_searching(false);
            }
            Message::CoverLoaded { book_id, image_data } => {
                if let Some(image) = Self::bytes_to_image(&image_data) {
                    ui.invoke_on_cover_loaded(SharedString::from(&book_id), image);
                }
            }
            Message::ChapterProgress { book_id, chapter_id, progress } => {
                // Update in memory
                let mut state = current_book_state.write().unwrap();
                if let Some(ref mut book) = *state {
                    if book.id == book_id {
                        if let Some(ch) = book.chapters.iter_mut().find(|c| c.id == chapter_id) {
                            ch.progress = progress;
                            ch.last_read = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0);
                        }

                        // Persist to database
                        if let Ok(db) = Database::new() {
                            let _ = db.save_full_book(book);
                        }
                    }
                }
            }
            Message::BookAdded(book) => {
                *current_book_state.write().unwrap() = Some(book.clone());
                ui.set_current_book(Self::book_to_slint(&book));

                // Refresh library
                if let Ok(db) = Database::new() {
                    if let Ok(books) = db.get_library_books() {
                        ui.set_library_books(Self::books_to_model(&books));
                    }
                }
            }
            Message::BookRemoved { book_id } => {
                // Update current book state
                let mut state = current_book_state.write().unwrap();
                if let Some(ref mut book) = *state {
                    if book.id == book_id {
                        book.in_library = false;
                    }
                }
                if let Some(book) = state.as_ref() {
                    ui.set_current_book(Self::book_to_slint(book));
                }

                // Refresh library
                if let Ok(db) = Database::new() {
                    if let Ok(books) = db.get_library_books() {
                        ui.set_library_books(Self::books_to_model(&books));
                    }
                }
            }
            Message::Error(err) => {
                ui.set_error_message(SharedString::from(err));
                ui.set_is_loading(false);
            }
            Message::ImportResult(ids) => {
                if ids.is_empty() {
                    ui.set_error_message(SharedString::from("No sources were imported."));
                } else {
                    let summary = ids.join(", ");
                    ui.set_error_message(SharedString::from(format!("Imported sources: {}", summary)));
                }
                ui.set_is_loading(false);
            }
        }
    }

    fn books_to_model(books: &[Book]) -> ModelRc<BookData> {
        let items: Vec<BookData> = books.iter().map(Self::book_to_slint).collect();
        ModelRc::new(VecModel::from(items))
    }

    fn sections_to_model(sections: &[HomeSection]) -> ModelRc<SectionData> {
        let items: Vec<SectionData> = sections.iter().map(Self::section_to_slint).collect();
        ModelRc::new(VecModel::from(items))
    }

    fn chapters_to_model(chapters: &[Chapter]) -> ModelRc<ChapterData> {
        let items: Vec<ChapterData> = chapters.iter().map(|c| ChapterData {
            id: SharedString::from(&c.id),
            title: SharedString::from(&c.title),
            date: SharedString::from(c.date.as_deref().unwrap_or("")),
            progress: c.progress,
        }).collect();
        ModelRc::new(VecModel::from(items))
    }

    fn search_results_to_model(results: &[SearchResult]) -> ModelRc<SearchResultData> {
        let items: Vec<SearchResultData> = results.iter().map(|r| SearchResultData {
            id: SharedString::from(&r.id),
            title: SharedString::from(&r.title),
            cover_url: SharedString::from(&r.cover_url),
            source_name: SharedString::from(r.source_name.as_deref().unwrap_or("")),
        }).collect();
        ModelRc::new(VecModel::from(items))
    }

    fn section_to_slint(section: &HomeSection) -> SectionData {
        let books: Vec<BookData> = section.books.iter().map(|b| BookData {
            id: SharedString::from(&b.id),
            title: SharedString::from(&b.title),
            author: SharedString::default(),
            cover_url: SharedString::from(&b.cover_url),
            progress: 0.0,
            chapters_count: 0,
            in_library: false,
        }).collect();

        SectionData {
            title: SharedString::from(&section.title),
            books: ModelRc::new(VecModel::from(books)),
        }
    }

    fn book_to_slint(book: &Book) -> BookData {
        let progress = if !book.chapters.is_empty() {
            let read = book.chapters.iter().filter(|c| c.progress > 0.5).count();
            read as f32 / book.chapters.len() as f32
        } else {
            0.0
        };

        BookData {
            id: SharedString::from(&book.id),
            title: SharedString::from(&book.title),
            author: SharedString::from(&book.author),
            cover_url: SharedString::from(&book.cover_url),
            progress,
            chapters_count: book.chapters_count,
            in_library: book.in_library,
        }
    }
}




