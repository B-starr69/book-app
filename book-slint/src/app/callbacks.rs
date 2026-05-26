use crate::App;
use book_core::{Book, Chapter, Database, SourceWithConfig};
use slint::{ComponentHandle, SharedString};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use super::cover_cache;
use super::messages::Message;
use super::SharedResources;

pub fn setup_callbacks(
    ui: &App,
    msg_tx: Sender<Message>,
    sources: Arc<RwLock<Vec<SourceWithConfig>>>,
    _database: Arc<Option<Database>>,
    current_book_state: Arc<RwLock<Option<Book>>>,
    shared: Arc<SharedResources>,
) {
    let ui_weak = ui.as_weak();
    ui.on_navigate(move |view| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_view(view);
        }
    });

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

    let msg_tx_discover = msg_tx.clone();
    let sources_discover = Arc::clone(&sources);
    let shared_discover = Arc::clone(&shared);
    ui.on_load_discover(move || {
        let tx = msg_tx_discover.clone();
        let sources = sources_discover.read().unwrap().clone();
        if let Some(source) = sources.first().cloned() {
            let source_id = source.id.clone();
            let shared = Arc::clone(&shared_discover);
            std::thread::spawn(move || {
                let rt = Arc::clone(&shared.runtime);
                rt.block_on(async {
                    if let Some(sections) = book_core::api::get_discover_page(source.clone()).await {
                        for section in &sections {
                            for book in &section.books {
                                super::load_cover_async(&tx, &source_id, &book.id, &book.cover_url, Arc::clone(&shared.http_client));
                            }
                        }
                        let _ = tx.send(Message::DiscoverLoaded { source_id, sections });
                    } else {
                        let _ = tx.send(Message::Error("Discover failed".to_string()));
                    }
                });
            });
        }
    });

    let ui_manage = ui.as_weak();
    ui.on_manage_sources(move || {
        if let Some(ui) = ui_manage.upgrade() {
            ui.set_show_import_dialog(true);
        }
    });

    let ui_close = ui.as_weak();
    ui.on_close_import_dialog(move || {
        if let Some(ui) = ui_close.upgrade() {
            ui.set_show_import_dialog(false);
            ui.set_import_repo_url(SharedString::from(""));
        }
    });

    let msg_tx_import = msg_tx.clone();
    let sources_import = Arc::clone(&sources);
    let shared_import = Arc::clone(&shared);
    ui.on_import_github(move |repo_url| {
        let tx = msg_tx_import.clone();
        let sources = Arc::clone(&sources_import);
        let shared = Arc::clone(&shared_import);
        let repo = repo_url.to_string();
        std::thread::spawn(move || {
            if let Ok(db) = Database::new() {
                match shared.runtime.block_on(async { book_core::import_from_github(&repo, &db).await }) {
                    Ok(imported) => {
                        if let Ok(new_sources) = db.get_sources_with_config() {
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

    let msg_tx_book = msg_tx.clone();
    let sources_book = Arc::clone(&sources);
    let shared_book = Arc::clone(&shared);
    ui.on_open_book(move |book_id| {
        let tx = msg_tx_book.clone();
        let sources = sources_book.read().unwrap().clone();
        let book_id = book_id.to_string();
        let shared = Arc::clone(&shared_book);
        if let Some(source) = sources.first().cloned() {
            std::thread::spawn(move || {
                if let Ok(db) = Database::new() {
                    if let Ok(Some(cached_book)) = db.get_full_book(&book_id, &source.id) {
                        let _ = tx.send(Message::BookDetailsLoaded(cached_book));
                        return;
                    }
                }

                let rt = Arc::clone(&shared.runtime);
                rt.block_on(async {
                    if let Some(details) = book_core::api::get_book_details(&source, book_id.clone()).await {
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
                            chapters: details
                                .chapters
                                .into_iter()
                                .map(|c| Chapter {
                                    id: c.id,
                                    title: c.title,
                                    date: c.date,
                                    progress: 0.0,
                                    last_read: 0,
                                })
                                .collect(),
                        };

                        if let Ok(db) = Database::new() {
                            let _ = db.save_full_book(&book);
                        }

                        let tx2 = tx.clone();
                        let source_id = source.id.clone();
                        let bid = book_id.clone();
                        let curl = details.cover_url.clone();
                        let client = Arc::clone(&shared.http_client);
                        std::thread::spawn(move || {
                            cover_cache::cache_cover_sync(&client, &source_id, &bid, &curl);
                            if let Some(path) = cover_cache::get_cached_cover_path(&source_id, &bid) {
                                if let Ok(data) = std::fs::read(&path) {
                                    let _ = tx2.send(Message::CoverLoaded {
                                        source_id: source_id.clone(),
                                        book_id: bid,
                                        image_data: data,
                                    });
                                }
                            }
                        });

                        let _ = tx.send(Message::BookDetailsLoaded(book));
                    }
                });
            });
        }
    });

    let msg_tx_chapter = msg_tx.clone();
    let sources_chapter = Arc::clone(&sources);
    let current_for_chapter = Arc::clone(&current_book_state);
    let shared_chapter = Arc::clone(&shared);
    ui.on_read_chapter(move |book_id, chapter_id| {
        let tx = msg_tx_chapter.clone();
        let sources = sources_chapter.read().unwrap().clone();
        let book_id = book_id.to_string();
        let chapter_id = chapter_id.to_string();
        let source_id = current_for_chapter
            .read()
            .unwrap()
            .as_ref()
            .map(|b| b.source_id.clone())
            .unwrap_or_default();
        if !source_id.is_empty() {
            let shared = Arc::clone(&shared_chapter);
            std::thread::spawn(move || {
                shared.runtime.block_on(async {
                    if let Some(source) = sources.iter().find(|s| s.id == source_id).cloned() {
                        if let Some(chapter) = book_core::api::get_chapter_content(&source, book_id.clone(), chapter_id.clone()).await {
                            let _ = tx.send(Message::ChapterContentLoaded {
                                content: chapter.content,
                                book_id,
                                chapter_id,
                            });
                        }
                    }
                });
            });
        }
    });

    let msg_tx_prev = msg_tx.clone();
    let sources_prev = Arc::clone(&sources);
    let current_prev = Arc::clone(&current_book_state);
    let ui_prev = ui.as_weak();
    let shared_prev = Arc::clone(&shared);
    ui.on_prev_chapter(move || {
        let Some(ui) = ui_prev.upgrade() else { return; };
        let current_chapter_id = ui.get_current_chapter_id().to_string();
        let Some(book) = current_prev.read().unwrap().clone() else { return; };
        let Some(index) = book.chapters.iter().position(|c| c.id == current_chapter_id) else { return; };
        if index == 0 {
            return;
        }
        let target = book.chapters[index - 1].id.clone();
        let source_id = book.source_id.clone();
        let book_id = book.id.clone();
        let sources = sources_prev.read().unwrap().clone();
        let tx = msg_tx_prev.clone();
        let shared = Arc::clone(&shared_prev);
        std::thread::spawn(move || {
            shared.runtime.block_on(async {
                if let Some(source) = sources.iter().find(|s| s.id == source_id).cloned() {
                    if let Some(chapter) = book_core::api::get_chapter_content(&source, book_id.clone(), target.clone()).await {
                        let _ = tx.send(Message::ChapterContentLoaded {
                            content: chapter.content,
                            book_id,
                            chapter_id: target,
                        });
                    }
                }
            });
        });
    });

    let msg_tx_next = msg_tx.clone();
    let sources_next = Arc::clone(&sources);
    let current_next = Arc::clone(&current_book_state);
    let ui_next = ui.as_weak();
    let shared_next = Arc::clone(&shared);
    ui.on_next_chapter(move || {
        let Some(ui) = ui_next.upgrade() else { return; };
        let current_chapter_id = ui.get_current_chapter_id().to_string();
        let Some(book) = current_next.read().unwrap().clone() else { return; };
        let Some(index) = book.chapters.iter().position(|c| c.id == current_chapter_id) else { return; };
        if index + 1 >= book.chapters.len() {
            return;
        }
        let target = book.chapters[index + 1].id.clone();
        let source_id = book.source_id.clone();
        let book_id = book.id.clone();
        let sources = sources_next.read().unwrap().clone();
        let tx = msg_tx_next.clone();
        let shared = Arc::clone(&shared_next);
        std::thread::spawn(move || {
            shared.runtime.block_on(async {
                if let Some(source) = sources.iter().find(|s| s.id == source_id).cloned() {
                    if let Some(chapter) = book_core::api::get_chapter_content(&source, book_id.clone(), target.clone()).await {
                        let _ = tx.send(Message::ChapterContentLoaded {
                            content: chapter.content,
                            book_id,
                            chapter_id: target,
                        });
                    }
                }
            });
        });
    });

    let msg_tx_progress = msg_tx.clone();
    ui.on_update_chapter_progress(move |book_id, chapter_id, progress| {
        let _ = msg_tx_progress.send(Message::ChapterProgress {
            book_id: book_id.to_string(),
            chapter_id: chapter_id.to_string(),
            progress,
        });
    });

    let msg_tx_add = msg_tx.clone();
    let current_add = Arc::clone(&current_book_state);
    ui.on_add_to_library(move || {
        let tx = msg_tx_add.clone();
        let current = Arc::clone(&current_add);
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

    let msg_tx_remove = msg_tx.clone();
    let current_remove = Arc::clone(&current_book_state);
    let sources_remove = Arc::clone(&sources);
    ui.on_remove_from_library(move || {
        let tx = msg_tx_remove.clone();
        let current = Arc::clone(&current_remove);
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

    let sources_search = Arc::clone(&sources);
    let shared_search = Arc::clone(&shared);
    ui.on_search(move |query| {
        let tx = msg_tx.clone();
        let sources = sources_search.read().unwrap().clone();
        let query = query.to_string();
        let shared = Arc::clone(&shared_search);
        std::thread::spawn(move || {
            shared.runtime.block_on(async {
                if let Some(source) = sources.first() {
                    if let Some(results) = book_core::api::search_books(source, &query).await {
                        for result in &results {
                            if let Some(result_source_id) = &result.source_id {
                                super::load_cover_async(&tx, result_source_id, &result.id, &result.cover_url, Arc::clone(&shared.http_client));
                            }
                        }
                        let _ = tx.send(Message::SearchResults(results));
                    }
                }
            });
        });
    });
}
