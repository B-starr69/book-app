use crate::App;
use book_core::{Book, Database, SourceWithConfig};
use slint::{ComponentHandle, SharedString, ModelRc, VecModel, Model};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use super::messages::Message;
use super::SharedResources;
use super::cover_registry;
use super::models;
use crate::BookData;

pub fn setup_callbacks(
    ui: &App,
    msg_tx: Sender<Message>,
    sources: Arc<RwLock<Vec<SourceWithConfig>>>,
    _database: Arc<Option<Database>>,
    current_book_state: Arc<RwLock<Option<Book>>>,
    shared: Arc<SharedResources>,
    library_books_state: Arc<RwLock<Vec<Book>>>,
    discover_sections_state: Arc<RwLock<Vec<book_core::HomeSection>>>,
    discover_source_id_state: Arc<RwLock<String>>,
    search_results_state: Arc<RwLock<Vec<book_core::SearchResult>>>,
    library_index_map: Arc<RwLock<HashMap<String, usize>>>,
    discover_index_map: Arc<RwLock<HashMap<String, (usize, usize)>>>,
    search_index_map: Arc<RwLock<HashMap<String, usize>>>,
) {
    let ui_weak = ui.as_weak();
    ui.on_navigate(move |view| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_view(view);
        }
    });

    // Update library model when a cover is loaded.
    let ui_cover_weak = ui.as_weak();
    let lib_state = Arc::clone(&library_books_state);
    let discover_state = Arc::clone(&discover_sections_state);
    let discover_source_id_state = Arc::clone(&discover_source_id_state);
    let search_state = Arc::clone(&search_results_state);
    let lib_index = Arc::clone(&library_index_map);
    let discover_index = Arc::clone(&discover_index_map);
    let search_index = Arc::clone(&search_index_map);
    ui.on_on_cover_loaded(move |book_id, _image| {
        if let Some(ui) = ui_cover_weak.upgrade() {
            let id = book_id.to_string();
            let books = lib_state.read().unwrap().clone();

            if let Some(pos) = lib_index.read().unwrap().get(&id).cloned() {
                if let Some(book) = books.get(pos) {
                    let pos_inner = pos;
                    let id_s = book.id.clone();
                    let source_id_s = book.source_id.clone();
                    let title_s = book.title.clone();
                    let author_s = book.author.clone();
                    let cover_url_s = book.cover_url.clone();
                    let progress = if !book.chapters.is_empty() {
                        let read = book.chapters.iter().filter(|c| c.progress > 0.5).count();
                        read as f32 / book.chapters.len() as f32
                    } else {
                        0.0
                    };
                    let chapters_count = book.chapters_count;
                    let in_library = book.in_library;
                    let ui_weak_inner = ui.as_weak();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_inner.upgrade() {
                            let cover_image = if let Some((rgba, w, h)) = cover_registry::get(&source_id_s, &id_s) {
                                models::rgba_to_image(&rgba, w, h).unwrap_or(slint::Image::default())
                            } else {
                                slint::Image::default()
                            };
                            let bd = crate::BookData {
                                id: SharedString::from(id_s.clone()),
                                source_id: SharedString::from(source_id_s.clone()),
                                title: SharedString::from(title_s.clone()),
                                author: SharedString::from(author_s.clone()),
                                cover_url: SharedString::from(cover_url_s.clone()),
                                progress,
                                chapters_count,
                                cover_image,
                                in_library,
                            };
                            ui.get_library_books().set_row_data(pos_inner, bd);
                        }
                    });
                }
            } else {
                let items: Vec<BookData> = books
                    .iter()
                    .map(|b| {
                        let mut bd = models::book_to_slint(b);
                        if b.id == id {
                            if let Some((rgba, w, h)) = cover_registry::get(&b.source_id, &b.id) {
                                bd.cover_image = models::rgba_to_image(&rgba, w, h).unwrap_or(slint::Image::default());
                            }
                        }
                        bd
                    })
                    .collect();
                ui.set_library_books(ModelRc::new(VecModel::from(items)));
            }

            if let Some((si, bi)) = discover_index.read().unwrap().get(&id).cloned() {
                let sections = discover_state.read().unwrap().clone();
                if let Some(section) = sections.get(si) {
                    if let Some(book) = section.books.get(bi) {
                        let source_id = discover_source_id_state.read().unwrap().clone();
                        let id_s = book.id.clone();
                        let title_s = book.title.clone();
                        let cover_url_s = book.cover_url.clone();
                        let ui_weak_inner = ui.as_weak();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_inner.upgrade() {
                                let cover_image = if let Some((rgba, w, h)) = cover_registry::get(&source_id, &id_s) {
                                    models::rgba_to_image(&rgba, w, h).unwrap_or(slint::Image::default())
                                } else {
                                    slint::Image::default()
                                };
                                let bd = crate::BookData {
                                    id: SharedString::from(&id_s),
                                    source_id: SharedString::from(&source_id),
                                    title: SharedString::from(&title_s),
                                    author: SharedString::default(),
                                    cover_url: SharedString::from(&cover_url_s),
                                    progress: 0.0,
                                    chapters_count: 0,
                                    cover_image,
                                    in_library: false,
                                };
                                if let Some(sec) = ui.get_discover_sections().row_data(si) {
                                    sec.books.set_row_data(bi, bd);
                                }
                            }
                        });
                    }
                }
            }

            if let Some(pos) = search_index.read().unwrap().get(&id).cloned() {
                let results = search_state.read().unwrap().clone();
                if let Some(result) = results.get(pos) {
                    let id_s = result.id.clone();
                    let source_id_s = result.source_id.clone();
                    let title_s = result.title.clone();
                    let cover_url_s = result.cover_url.clone();
                    let source_name_s = result.source_name.clone();
                    let ui_weak_inner = ui.as_weak();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_inner.upgrade() {
                            let cover_image = if let Some(src_id) = source_id_s.as_ref() {
                                if let Some((rgba, w, h)) = cover_registry::get(src_id, &id_s) {
                                    models::rgba_to_image(&rgba, w, h).unwrap_or(slint::Image::default())
                                } else {
                                    slint::Image::default()
                                }
                            } else {
                                slint::Image::default()
                            };
                            let sr = crate::SearchResultData {
                                id: SharedString::from(&id_s),
                                source_id: source_id_s.as_ref().map_or_else(SharedString::default, SharedString::from),
                                title: SharedString::from(&title_s),
                                cover_url: SharedString::from(&cover_url_s),
                                source_name: source_name_s.as_ref().map_or_else(SharedString::default, SharedString::from),
                                cover_image,
                            };
                            ui.get_search_results().set_row_data(pos, sr);
                        }
                    });
                }
            }

            let sections = discover_state.read().unwrap().clone();
            if !sections.is_empty() {
                let source_id = discover_source_id_state.read().unwrap().clone();
                ui.set_discover_sections(models::sections_to_model(&sections, &source_id));
            }

            let results = search_state.read().unwrap().clone();
            if !results.is_empty() {
                ui.set_search_results(models::search_results_to_model(&results));
            }
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
    ui.on_load_discover(move |source_id| {
        let tx = msg_tx_discover.clone();
        let sources = sources_discover.read().unwrap().clone();
        let source_id = source_id.to_string();
        let source = sources
            .iter()
            .find(|candidate| candidate.id == source_id)
            .cloned()
            .or_else(|| sources.first().cloned());
        if let Some(source) = source {
            let source_id = source.id.clone();
            let shared = Arc::clone(&shared_discover);
            std::thread::spawn(move || {
                let rt = Arc::clone(&shared.runtime);
                rt.block_on(async {
                    if let Some(sections) = book_core::api::get_discover_page(source.clone()).await {
                        for section in &sections {
                            for book in &section.books {
                                super::load_cover_async(
                                    &tx,
                                    &source_id,
                                    &book.id,
                                    &book.cover_url,
                                    Arc::clone(&shared.runtime),
                                );
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
    ui.on_open_book(move |book_id, source_id| {
        let tx = msg_tx_book.clone();
        let sources = sources_book.read().unwrap().clone();
        let book_id = book_id.to_string();
        let source_id = source_id.to_string();
        let shared = Arc::clone(&shared_book);
        let source = sources
            .iter()
            .find(|candidate| candidate.id == source_id)
            .cloned()
            .or_else(|| sources.first().cloned());
        if let Some(source) = source {
            std::thread::spawn(move || {
                let rt = Arc::clone(&shared.runtime);
                let book = if let Ok(db) = Database::new() {
                    rt.block_on(async {
                        book_core::api::get_book_details_cached(&db, &source, book_id.clone()).await
                    })
                } else {
                    None
                };

                if let Some(book) = book {
                    let _ = tx.send(Message::BookDetailsLoaded(book.clone()));
                    super::load_cover_async(
                        &tx,
                        &book.source_id,
                        &book.id,
                        &book.cover_url,
                        Arc::clone(&shared.runtime),
                    );
                } else {
                    let _ = tx.send(Message::Error("Book details failed".to_string()));
                }
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
                if let Ok(db) = Database::new() {
                    let content = shared.runtime.block_on(async {
                        if let Some(source) = sources.iter().find(|s| s.id == source_id).cloned() {
                            book_core::api::get_chapter_content_cached(
                                &db,
                                &source,
                                book_id.clone(),
                                chapter_id.clone(),
                            )
                            .await
                        } else {
                            None
                        }
                    });

                    if let Some(content) = content {
                        let _ = tx.send(Message::ChapterContentLoaded {
                            content,
                            book_id,
                            chapter_id,
                        });
                    }
                }
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
            if let Ok(db) = Database::new() {
                let content = shared.runtime.block_on(async {
                    if let Some(source) = sources.iter().find(|s| s.id == source_id).cloned() {
                        book_core::api::get_chapter_content_cached(&db, &source, book_id.clone(), target.clone()).await
                    } else {
                        None
                    }
                });

                if let Some(content) = content {
                    let _ = tx.send(Message::ChapterContentLoaded {
                        content,
                        book_id,
                        chapter_id: target,
                    });
                }
            }
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
            if let Ok(db) = Database::new() {
                let content = shared.runtime.block_on(async {
                    if let Some(source) = sources.iter().find(|s| s.id == source_id).cloned() {
                        book_core::api::get_chapter_content_cached(&db, &source, book_id.clone(), target.clone()).await
                    } else {
                        None
                    }
                });

                if let Some(content) = content {
                    let _ = tx.send(Message::ChapterContentLoaded {
                        content,
                        book_id,
                        chapter_id: target,
                    });
                }
            }
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

    let msg_tx_search = msg_tx.clone();
    let sources_search = Arc::clone(&sources);
    let shared_search = Arc::clone(&shared);
    ui.on_search(move |query| {
        let tx = msg_tx_search.clone();
        let sources = sources_search.read().unwrap().clone();
        let query = query.to_string();
        let shared = Arc::clone(&shared_search);
        std::thread::spawn(move || {
            shared.runtime.block_on(async {
                if let Some(source) = sources.first() {
                    if let Some(results) = book_core::api::search_books(source, &query).await {
                        for result in &results {
                            if let Some(result_source_id) = &result.source_id {
                                super::load_cover_async(
                                    &tx,
                                    result_source_id,
                                    &result.id,
                                    &result.cover_url,
                                    Arc::clone(&shared.runtime),
                                );
                            }
                        }
                        let _ = tx.send(Message::SearchResults(results));
                    }
                }
            });
        });
    });

    let msg_tx_cache = msg_tx.clone();
    ui.on_clear_cache(move || {
        let tx = msg_tx_cache.clone();
        std::thread::spawn(move || {
            if let Ok(db) = Database::new() {
                let _ = db.clear_all_cache();
            }
            let _ = tx.send(Message::Error("Cache cleared".to_string()));
        });
    });
}
