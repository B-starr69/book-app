#[path = "messages.rs"]
mod messages;
#[path = "models.rs"]
mod models;
#[path = "callbacks.rs"]
mod callbacks;
#[path = "cover_registry.rs"]
mod cover_registry;

use crate::{App, ViewState};
use book_core::{Book, Database, SourceWithConfig};
use messages::Message;
use slint::{ComponentHandle, Image, SharedString};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

pub struct BookApp {
    ui: App,
}

fn sync_discover_sources(ui: &App, sources: &[SourceWithConfig]) {
    let items: Vec<crate::SourceData> = sources
        .iter()
        .map(|source| crate::SourceData {
            id: SharedString::from(&source.id),
            name: SharedString::from(&source.name),
        })
        .collect();
    let current_selected = ui.get_selected_discover_source_id().to_string();
    let selected = if sources.iter().any(|source| source.id == current_selected) {
        current_selected
    } else {
        sources.first().map(|source| source.id.clone()).unwrap_or_default()
    };
    ui.set_discover_sources(slint::ModelRc::new(slint::VecModel::from(items)));
    ui.set_selected_discover_source_id(SharedString::from(selected.as_str()));
}


pub(super) struct SharedResources {
    runtime: Arc<tokio::runtime::Runtime>,
}

impl BookApp {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = App::new()?;
        let (msg_tx, msg_rx) = channel::<Message>();

        let database = Arc::new(Database::new().ok());
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let shared = Arc::new(SharedResources {
            runtime: Arc::clone(&runtime),
        });

        let sources = if let Some(ref db) = *database {
            db.get_sources_with_config().unwrap_or_default()
        } else {
            vec![]
        };
        sync_discover_sources(&ui, &sources);
        let sources = Arc::new(RwLock::new(sources));
        let library_books_state = Arc::new(RwLock::new(Vec::<Book>::new()));
        let discover_sections_state = Arc::new(RwLock::new(Vec::<book_core::HomeSection>::new()));
        let discover_source_id_state = Arc::new(RwLock::new(String::new()));
        let search_results_state = Arc::new(RwLock::new(Vec::<book_core::SearchResult>::new()));

        // Index maps to support O(1) lookups for incremental updates
        let library_index_map = Arc::new(RwLock::new(HashMap::<String, usize>::new()));
        let discover_index_map = Arc::new(RwLock::new(HashMap::<String, (usize, usize)>::new()));
        let search_index_map = Arc::new(RwLock::new(HashMap::<String, usize>::new()));

        if let Some(ref db) = *database {
            if let Ok(books) = db.get_library_books() {
                // Populate library model, using cover_registry when possible to avoid re-decoding.
                let mut items = Vec::new();
                for book in books.iter() {
                    let mut bd = models::book_to_slint(book);
                    if let Some((rgba_arc, width, height)) = cover_registry::get(&book.source_id, &book.id) {
                        if let Some(img) = models::rgba_to_image(&*rgba_arc, width, height) {
                            bd.cover_image = img;
                        }
                    }
                    items.push(bd);
                }
                ui.set_library_books(slint::ModelRc::new(slint::VecModel::from(items)));
                // keep a copy of the library books for UI updates when covers arrive
                *library_books_state.write().unwrap() = books.clone();
                // build index map
                {
                    let mut map = library_index_map.write().unwrap();
                    map.clear();
                    for (i, b) in books.iter().enumerate() {
                        map.insert(b.id.clone(), i);
                    }
                }
                let tx = msg_tx.clone();
                for book in books.iter() {
                    load_cover_async(&tx, &book.source_id, &book.id, &book.cover_url, Arc::clone(&runtime));
                }
            }
        }

        let current_book_state = Arc::new(RwLock::new(None::<Book>));

        callbacks::setup_callbacks(
            &ui,
            msg_tx.clone(),
            Arc::clone(&sources),
            Arc::clone(&database),
            Arc::clone(&current_book_state),
            Arc::clone(&shared),
            Arc::clone(&library_books_state),
            Arc::clone(&discover_sections_state),
            Arc::clone(&discover_source_id_state),
            Arc::clone(&search_results_state),
            Arc::clone(&library_index_map),
            Arc::clone(&discover_index_map),
            Arc::clone(&search_index_map),
        );

        let ui_weak = ui.as_weak();
        let timer = slint::Timer::default();
        let current_book_for_timer = Arc::clone(&current_book_state);
        let library_books_for_timer = Arc::clone(&library_books_state);
        let discover_sections_for_timer = Arc::clone(&discover_sections_state);
        let discover_source_id_for_timer = Arc::clone(&discover_source_id_state);
        let search_results_for_timer = Arc::clone(&search_results_state);
        let library_index_for_timer = Arc::clone(&library_index_map);
        let discover_index_for_timer = Arc::clone(&discover_index_map);
        let search_index_for_timer = Arc::clone(&search_index_map);
        timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50),
            move || {
                while let Ok(msg) = msg_rx.try_recv() {
                    if let Some(ui) = ui_weak.upgrade() {
                        Self::handle_message(&ui, msg, &sources, &current_book_for_timer, &library_books_for_timer, &discover_sections_for_timer, &discover_source_id_for_timer, &search_results_for_timer, &library_index_for_timer, &discover_index_for_timer, &search_index_for_timer);
                    }
                }
            },
        );
        std::mem::forget(timer);

        Ok(Self { ui })
    }

    pub fn run(self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }

        fn handle_message(
        ui: &App,
        msg: Message,
        sources: &Arc<RwLock<Vec<SourceWithConfig>>>,
        current_book_state: &Arc<RwLock<Option<Book>>>,
        library_books_state: &Arc<RwLock<Vec<Book>>>,
        discover_sections_state: &Arc<RwLock<Vec<book_core::HomeSection>>>,
        discover_source_id_state: &Arc<RwLock<String>>,
        search_results_state: &Arc<RwLock<Vec<book_core::SearchResult>>>,
        library_index_map: &Arc<RwLock<HashMap<String, usize>>>,
        discover_index_map: &Arc<RwLock<HashMap<String, (usize, usize)>>>,
        search_index_map: &Arc<RwLock<HashMap<String, usize>>>,
    ) {
        match msg {
            Message::LibraryLoaded(books) => {
                ui.set_library_books(models::books_to_model(&books));
                // keep a shared copy for cover updates
                *library_books_state.write().unwrap() = books.clone();
                // rebuild index map
                {
                    let mut map = library_index_map.write().unwrap();
                    map.clear();
                    for (i, b) in books.iter().enumerate() {
                        map.insert(b.id.clone(), i);
                    }
                }
                ui.set_is_loading(false);
            }
            Message::DiscoverLoaded { source_id, sections } => {
                // save sections and source id for later incremental updates and set UI model (models will consult registry)
                *discover_sections_state.write().unwrap() = sections.clone();
                *discover_source_id_state.write().unwrap() = source_id.clone();
                ui.set_selected_discover_source_id(SharedString::from(&source_id));
                // rebuild discover index map (section_idx, book_idx)
                {
                    let mut map = discover_index_map.write().unwrap();
                    map.clear();
                    for (si, section) in sections.iter().enumerate() {
                        for (bi, book) in section.books.iter().enumerate() {
                            map.insert(book.id.clone(), (si, bi));
                        }
                    }
                }
                ui.set_discover_sections(models::sections_to_model(&sections, &source_id));
                ui.set_is_loading(false);
            }
            Message::BookDetailsLoaded(book) => {
                let mut merged_book = book.clone();
                if let Ok(db) = Database::new() {
                    if let Ok(db_chapters) = db.get_chapters_for_book(&merged_book.id, &merged_book.source_id) {
                        for ch in merged_book.chapters.iter_mut() {
                            if let Some(dbc) = db_chapters.iter().find(|d| d.id == ch.id) {
                                ch.progress = dbc.progress;
                                ch.last_read = dbc.last_read;
                            }
                        }
                    }
                }

                *current_book_state.write().unwrap() = Some(merged_book.clone());
                ui.set_current_book(models::book_to_slint(&merged_book));
                ui.set_book_chapters(models::chapters_to_model(&merged_book.chapters));
                // Prefer in-memory registry (RGBA) to avoid decoding on UI thread
                if let Some((rgba_arc, w, h)) = cover_registry::get(&merged_book.source_id, &merged_book.id) {
                    if let Some(image) = models::rgba_to_image(&*rgba_arc, w, h) {
                        ui.set_current_book_cover(image);
                    } else {
                        ui.set_current_book_cover(Image::default());
                    }
                } else {
                    let cached_bytes = if let Ok(db) = Database::new() {
                        db.get_cached_cover(&merged_book.id, &merged_book.source_id).ok().flatten()
                    } else {
                        None
                    };

                    if let Some(bytes) = cached_bytes {
                        if let Some(image) = models::bytes_to_image(&bytes) {
                            ui.set_current_book_cover(image);
                        } else {
                            ui.set_current_book_cover(Image::default());
                        }
                    } else {
                        ui.set_current_book_cover(Image::default());
                    }
                }
                ui.set_current_view(ViewState::BookDetails);
                ui.set_is_loading(false);
            }
            Message::ChapterContentLoaded { content, book_id, chapter_id } => {
                let chapter_progress = if let Some(book) = current_book_state.read().unwrap().as_ref() {
                    book.chapters
                        .iter()
                        .find(|chapter| chapter.id == chapter_id)
                        .map(|chapter| chapter.progress.clamp(0.0, 1.0))
                        .unwrap_or(0.0)
                } else {
                    0.0
                };
                ui.set_reader_progress(0.0);
                ui.set_chapter_content(SharedString::from(&content));
                ui.set_current_chapter_id(SharedString::from(&chapter_id));
                ui.set_current_book_id(SharedString::from(&book_id));
                ui.set_current_view(ViewState::Reader);

                let ui_weak = ui.as_weak();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_reader_progress(chapter_progress);
                    }
                });
                ui.set_is_loading(false);
            }
            Message::SearchResults(results) => {
                *search_results_state.write().unwrap() = results.clone();
                // rebuild search index map
                {
                    let mut map = search_index_map.write().unwrap();
                    map.clear();
                    for (i, r) in results.iter().enumerate() {
                        map.insert(r.id.clone(), i);
                    }
                }
                ui.set_is_searching(false);
            }
                    Message::CoverLoaded { source_id, book_id, image_data } => {
                        // Backwards-compatible: if someone still sends raw bytes, decode and thumbnail on the UI thread.
                        if let Ok(img) = image::load_from_memory(&image_data) {
                            let thumb = img.thumbnail(256, 256);
                            let rgba_buf = thumb.to_rgba8();
                            let width = rgba_buf.width();
                            let height = rgba_buf.height();
                            let rgba_vec = rgba_buf.into_raw();
                            if let Some(image) = models::rgba_to_image(&rgba_vec, width, height) {
                                if current_book_state
                                    .read()
                                    .unwrap()
                                    .as_ref()
                                    .map(|book| book.id == book_id && book.source_id == source_id)
                                    .unwrap_or(false)
                                {
                                    ui.set_current_book_cover(image.clone());
                                }
                                // Insert RGBA bytes into the registry (Send/Sync-friendly)
                                cover_registry::insert(&source_id, &book_id, std::sync::Arc::new(rgba_vec.clone()), width, height);
                                // Notify UI with the slint Image for immediate display
                                ui.invoke_on_cover_loaded(SharedString::from(&book_id), image);
                            }
                        }
                    }
                    Message::CoverDecoded { source_id, book_id, rgba, width, height } => {
                        if let Some(image) = models::rgba_to_image(&rgba, width, height) {
                            if current_book_state
                                .read()
                                .unwrap()
                                .as_ref()
                                .map(|book| book.id == book_id && book.source_id == source_id)
                                .unwrap_or(false)
                            {
                                ui.set_current_book_cover(image.clone());
                            }
                            // store raw rgba bytes (sendable) in the registry
                            cover_registry::insert(&source_id, &book_id, std::sync::Arc::new(rgba.clone()), width, height);
                            ui.invoke_on_cover_loaded(SharedString::from(&book_id), image);
                        }
                    }
            Message::ChapterProgress { book_id, chapter_id, progress } => {
                let mut state = current_book_state.write().unwrap();
                if let Some(ref mut book) = *state {
                    if book.id == book_id {
                        if let Some(ch) = book.chapters.iter_mut().find(|c| c.id == chapter_id) {
                            let p = progress.clamp(0.0, 1.0);
                            ch.progress = p;
                            ch.last_read = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0);
                        }
                        if let Ok(db) = Database::new() {
                            let _ = db.save_full_book(book);
                        }
                    }
                }
                ui.set_reader_progress(progress.clamp(0.0, 1.0));
            }
            Message::BookAdded(book) => {
                *current_book_state.write().unwrap() = Some(book.clone());
                ui.set_current_book(models::book_to_slint(&book));
                if let Ok(db) = Database::new() {
                    if let Ok(books) = db.get_library_books() {
                        ui.set_library_books(models::books_to_model(&books));
                    }
                }
            }
            Message::BookRemoved { book_id } => {
                let mut state = current_book_state.write().unwrap();
                if let Some(ref mut book) = *state {
                    if book.id == book_id {
                        book.in_library = false;
                    }
                }
                if let Some(book) = state.as_ref() {
                    ui.set_current_book(models::book_to_slint(book));
                }
                if let Ok(db) = Database::new() {
                    if let Ok(books) = db.get_library_books() {
                        ui.set_library_books(models::books_to_model(&books));
                    }
                }
            }
            Message::Error(err) => {
                ui.set_error_message(SharedString::from(err));
                ui.set_is_loading(false);
            }
            Message::ImportResult(ids) => {
                if let Ok(sources_guard) = sources.read() {
                    sync_discover_sources(ui, &sources_guard);
                }
                if ids.is_empty() {
                    ui.set_error_message(SharedString::from("No sources were imported."));
                } else {
                    ui.set_error_message(SharedString::from(format!("Imported sources: {}", ids.join(", "))));
                }
                ui.set_is_loading(false);
            }
        }
    }
}

pub(super) fn load_cover_async(
    tx: &std::sync::mpsc::Sender<Message>,
    source_id: &str,
    book_id: &str,
    cover_url: &str,
    runtime: Arc<tokio::runtime::Runtime>,
) {
    let source_id = source_id.to_string();
    let book_id = book_id.to_string();
    let cover_url = cover_url.to_string();
    let tx = tx.clone();

    let runtime_for_blocking = Arc::clone(&runtime);
    runtime_for_blocking.spawn_blocking(move || {
        if let Ok(db) = Database::new() {
            if let Some(bytes) = book_core::api::get_cover_bytes_cached_blocking(
                &db,
                &cover_url,
                &source_id,
                &book_id,
            ) {
                // Decode and create a small thumbnail on the background thread to avoid blocking the UI.
                if let Ok(img) = image::load_from_memory(&bytes) {
                    // Create a thumbnail with a max dimension (keeps aspect ratio).
                    let thumb = img.thumbnail(256, 256);
                    let rgba = thumb.to_rgba8();
                    let width = rgba.width();
                    let height = rgba.height();
                    let rgba_vec = rgba.into_raw();
                    let _ = tx.send(Message::CoverDecoded { source_id, book_id, rgba: rgba_vec, width, height });
                } else {
                    // Fallback: send raw bytes if decoding failed in background.
                    let _ = tx.send(Message::CoverLoaded {
                        source_id,
                        book_id,
                        image_data: bytes,
                    });
                }
            }
        }
    });
}
