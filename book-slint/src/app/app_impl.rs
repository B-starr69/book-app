#[path = "messages.rs"]
mod messages;
#[path = "cover_cache.rs"]
mod cover_cache;
#[path = "models.rs"]
mod models;
#[path = "callbacks.rs"]
mod callbacks;

use crate::{App, ViewState};
use book_core::{Book, Database};
use messages::Message;
use slint::{ComponentHandle, Image, SharedString};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};

pub struct BookApp {
    ui: App,
}

pub(super) struct SharedResources {
    runtime: Arc<tokio::runtime::Runtime>,
    http_client: Arc<reqwest::blocking::Client>,
}

impl BookApp {
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = App::new()?;
        let (msg_tx, msg_rx) = channel::<Message>();

        let database = Arc::new(Database::new().ok());
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let http_client = Arc::new(
            reqwest::blocking::Client::builder()
                .user_agent("book-app")
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
        );
        let shared = Arc::new(SharedResources {
            runtime: Arc::clone(&runtime),
            http_client: Arc::clone(&http_client),
        });

        let sources = if let Some(ref db) = *database {
            db.get_sources_with_config().unwrap_or_default()
        } else {
            vec![]
        };
        let sources = Arc::new(RwLock::new(sources));

        if let Some(ref db) = *database {
            if let Ok(books) = db.get_library_books() {
                ui.set_library_books(models::books_to_model(&books));
                let tx = msg_tx.clone();
                for book in books.iter() {
                    load_cover_async(&tx, &book.source_id, &book.id, &book.cover_url, Arc::clone(&http_client));
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
        );

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

    pub fn run(self) -> Result<(), slint::PlatformError> {
        self.ui.run()
    }

    fn handle_message(ui: &App, msg: Message, current_book_state: &Arc<RwLock<Option<Book>>>) {
        match msg {
            Message::LibraryLoaded(books) => {
                ui.set_library_books(models::books_to_model(&books));
                ui.set_is_loading(false);
            }
            Message::DiscoverLoaded { source_id, sections } => {
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
                if let Some(path) = cover_cache::get_cached_cover_path(&merged_book.source_id, &merged_book.id) {
                    if let Ok(data) = std::fs::read(&path) {
                        if let Some(image) = models::bytes_to_image(&data) {
                            ui.set_current_book_cover(image);
                        } else {
                            ui.set_current_book_cover(Image::default());
                        }
                    } else {
                        ui.set_current_book_cover(Image::default());
                    }
                } else {
                    ui.set_current_book_cover(Image::default());
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
                ui.set_search_results(models::search_results_to_model(&results));
                ui.set_is_searching(false);
            }
            Message::CoverLoaded { source_id, book_id, image_data } => {
                if let Some(image) = models::bytes_to_image(&image_data) {
                    if current_book_state
                        .read()
                        .unwrap()
                        .as_ref()
                        .map(|book| book.id == book_id && book.source_id == source_id)
                        .unwrap_or(false)
                    {
                        ui.set_current_book_cover(image.clone());
                    }
                    models::update_book_cover_models(ui, &source_id, &book_id, image.clone());
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
    http_client: Arc<reqwest::blocking::Client>,
) {
    let source_id = source_id.to_string();
    let book_id = book_id.to_string();
    let cover_url = cover_url.to_string();
    let tx = tx.clone();

    std::thread::spawn(move || {
        if let Some(path) = cover_cache::get_cached_cover_path(&source_id, &book_id) {
            if let Ok(data) = std::fs::read(&path) {
                let _ = tx.send(Message::CoverLoaded {
                    source_id,
                    book_id,
                    image_data: data,
                });
                return;
            }
        }

        if !cover_url.is_empty() {
            cover_cache::cache_cover_sync(&http_client, &source_id, &book_id, &cover_url);
            if let Some(path) = cover_cache::get_cached_cover_path(&source_id, &book_id) {
                if let Ok(data) = std::fs::read(&path) {
                    let _ = tx.send(Message::CoverLoaded {
                        source_id,
                        book_id,
                        image_data: data,
                    });
                }
            }
        }
    });
}
