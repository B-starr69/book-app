use crate::cover_cache::cache_cover_sync;
use crate::state::{AppState, Command, StreamingState};
use book_core::{api, Book, Chapter, Database, HomeSection};
use eframe::egui::Context;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, RwLock};

/// Run the logic thread - handles all async operations
pub fn run_logic_thread(
    state: Arc<RwLock<AppState>>,
    cmd_rx: Receiver<Command>,
    ctx: Context,
    db_path: Option<String>,
) {
    // Create a single tokio runtime for all async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    // Open database connection for this thread
    let database = db_path.and_then(|_| Database::new().ok());

    // Process commands in a loop
    loop {
        // Wait for a command (blocking)
        match cmd_rx.recv() {
            Ok(cmd) => {
                // Process command
                rt.block_on(async {
                    process_command(&state, &database, cmd, &ctx).await;
                });
                // Request UI repaint after processing
                ctx.request_repaint();
            }
            Err(_) => {
                // Channel closed, exit thread
                println!("[LOGIC] Command channel closed, exiting logic thread");
                break;
            }
        }
    }
}

/// Process a single command
async fn process_command(
    state: &Arc<RwLock<AppState>>,
    database: &Option<Database>,
    cmd: Command,
    ctx: &Context,
) {
    match cmd {
        Command::LoadDiscover => {
            let source = {
                let s = state.read().unwrap();
                s.selected_source.clone()
            };

            if let Some(source) = source {
                // Set streaming loading state and clear old sections
                {
                    let mut s = state.write().unwrap();
                    s.is_loading = true;
                    s.discover_streaming = StreamingState::Loading { items_loaded: 0 };
                    s.discover_sections.clear();
                    s.error_message = None;
                }
                ctx.request_repaint();

                // Create channel for streaming sections
                let (section_tx, section_rx) = channel::<HomeSection>();
                let state_clone = Arc::clone(state);
                let ctx_clone = ctx.clone();
                let source_id = source.id.clone();

                // Spawn a thread to receive streaming sections and update state
                let receiver_handle = std::thread::spawn(move || {
                    let mut count = 0;
                    while let Ok(section) = section_rx.recv() {
                        // Cache covers for this section
                        for book in &section.books {
                            if !book.cover_url.is_empty() {
                                let sid = source_id.clone();
                                let bid = book.id.clone();
                                let curl = book.cover_url.clone();
                                std::thread::spawn(move || {
                                    cache_cover_sync(&sid, &bid, &curl);
                                });
                            }
                        }

                        // Append section to state immediately
                        {
                            let mut s = state_clone.write().unwrap();
                            s.discover_sections.push(section);
                            count += 1;
                            s.discover_streaming = StreamingState::Loading { items_loaded: count };
                        }
                        // Request repaint so UI shows the new section
                        ctx_clone.request_repaint();
                    }
                    count
                });

                // Start streaming (this sends sections as they parse)
                let result = api::get_discover_page_streaming(source.clone(), section_tx).await;

                // Wait for receiver to finish processing all sections
                let _ = receiver_handle.join();

                // Update final state
                {
                    let mut s = state.write().unwrap();
                    s.is_loading = false;
                    match result {
                        Some(_) => {
                            s.discover_streaming = StreamingState::Done;
                        }
                        None => {
                            s.discover_streaming = StreamingState::Error("Failed to load discover page".to_string());
                            s.error_message = Some("Failed to load discover page".to_string());
                        }
                    }
                }
            }
        }

        Command::LoadBookDetails { book_id } => {
            let (source, library_books) = {
                let s = state.read().unwrap();
                (s.selected_source.clone(), s.library_books.clone())
            };

            if let Some(source) = source {
                // Check cache first
                let use_cache = if let Some(ref db) = database {
                    let needs_sync = db.needs_sync(&book_id, &source.id, 7);
                    if !needs_sync {
                        if let Ok(Some(mut cached_book)) = db.get_full_book(&book_id, &source.id) {
                            cached_book.in_library = library_books
                                .iter()
                                .any(|b| b.id == cached_book.id && b.source_id == cached_book.source_id);
                            let mut s = state.write().unwrap();
                            s.current_book = Some(cached_book);
                            s.is_loading = false;
                            s.book_details_streaming = StreamingState::Done;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !use_cache {
                    // Set loading state
                    {
                        let mut s = state.write().unwrap();
                        s.is_loading = true;
                        s.book_details_streaming = StreamingState::Loading { items_loaded: 0 };
                        s.current_book = None;
                        s.error_message = None;
                    }
                    ctx.request_repaint();

                    // Step 1: Get metadata only (returns immediately)
                    match api::get_book_metadata_only(&source, book_id.clone()).await {
                        Some(details) => {
                            let cover_url = details.cover_url.clone();
                            let chapters_count = details.chapters_count;

                            // Step 2: Set book metadata immediately (show UI)
                            {
                                let mut s = state.write().unwrap();
                                let in_library = s
                                    .library_books
                                    .iter()
                                    .any(|b| b.id == book_id && b.source_id == source.id);

                                let book = Book {
                                    id: book_id.clone(),
                                    source_id: source.id.clone(),
                                    title: details.title,
                                    author: details.author,
                                    cover_url: cover_url.clone(),
                                    rating: details.rating,
                                    status: details.status,
                                    chapters_count: details.chapters_count,
                                    genres: details.genres,
                                    summary: details.summary,
                                    in_library,
                                    chapters: vec![], // Will be filled by streaming
                                };
                                s.current_book = Some(book);
                                s.is_loading = false; // Show metadata immediately
                            }
                            ctx.request_repaint();

                            // Cache cover in background
                            if !cover_url.is_empty() {
                                let sid = source.id.clone();
                                let bid = book_id.clone();
                                let curl = cover_url;
                                std::thread::spawn(move || {
                                    cache_cover_sync(&sid, &bid, &curl);
                                });
                            }

                            // Step 3: Create channel and receiver for streaming chapters
                            let (chapters_tx, chapters_rx) = channel::<Vec<book_core::ParsedChapterInfo>>();
                            let state_clone = Arc::clone(state);
                            let ctx_clone = ctx.clone();
                            let book_id_clone = book_id.clone();

                            // Spawn receiver thread to update UI as chapters arrive
                            std::thread::spawn(move || {
                                let mut total_chapters = 0;

                                while let Ok(chapter_batch) = chapters_rx.recv() {
                                    let batch_size = chapter_batch.len();
                                    total_chapters += batch_size;

                                    // Append chapters to current book
                                    {
                                        let mut s = state_clone.write().unwrap();
                                        if let Some(ref mut book) = s.current_book {
                                            for ch in chapter_batch {
                                                book.chapters.push(Chapter {
                                                    id: ch.id,
                                                    title: ch.title,
                                                    date: ch.date,
                                                    progress: 0.0,
                                                    last_read: 0,
                                                });
                                            }
                                        }
                                        s.book_details_streaming = StreamingState::Loading { items_loaded: total_chapters };
                                    }
                                    ctx_clone.request_repaint();
                                }

                                // Mark streaming as done
                                {
                                    let mut s = state_clone.write().unwrap();
                                    s.book_details_streaming = StreamingState::Done;
                                    s.needs_save_after_streaming = true;
                                }
                                ctx_clone.request_repaint();
                            });

                            // Step 4: Start chapter streaming (spawns thread, returns immediately)
                            api::start_chapter_streaming(
                                source.clone(),
                                book_id_clone,
                                chapters_count,
                                chapters_tx,
                            );
                        }
                        None => {
                            let mut s = state.write().unwrap();
                            s.error_message = Some("Failed to load book details".to_string());
                            s.is_loading = false;
                            s.book_details_streaming = StreamingState::Error("Failed to load".to_string());
                        }
                    }
                }
            }
        }

        Command::LoadChapter { book_id, chapter_id } => {
            let source = {
                let s = state.read().unwrap();
                s.selected_source.clone()
            };

            if let Some(source) = source {
                // Check cache first
                let use_cache = if let Some(ref db) = database {
                    if let Ok(Some(cached_content)) =
                        db.get_cached_chapter_content(&book_id, &source.id, &chapter_id)
                    {
                        let mut s = state.write().unwrap();
                        s.chapter_content = cached_content;
                        s.is_loading = false;
                        // Don't set progress here - let the reader track scroll position
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !use_cache {
                    {
                        let mut s = state.write().unwrap();
                        s.is_loading = true;
                        s.error_message = None;
                    }
                    ctx.request_repaint();

                    match api::get_chapter_content(&source, book_id.clone(), chapter_id.clone())
                        .await
                    {
                        Some(chapter) => {
                            // Cache content
                            if let Some(ref db) = database {
                                let _ = db.cache_chapter_content(
                                    &book_id,
                                    &source.id,
                                    &chapter_id,
                                    &chapter.content,
                                );
                            }

                            let mut s = state.write().unwrap();
                            s.chapter_content = chapter.content;
                            s.is_loading = false;
                            // Don't set progress here - let the reader track scroll position
                        }
                        None => {
                            let mut s = state.write().unwrap();
                            s.error_message = Some("Failed to load chapter".to_string());
                            s.is_loading = false;
                        }
                    }
                }
            }
        }

        Command::RefreshCurrentBook => {
            let (book, sources) = {
                let s = state.read().unwrap();
                (s.current_book.clone(), s.sources.clone())
            };

            if let Some(book) = book {
                if let Some(source) = sources.iter().find(|s| s.id == book.source_id) {
                    {
                        let mut s = state.write().unwrap();
                        s.is_loading = true;
                        s.error_message = None;
                    }
                    ctx.request_repaint();

                    // Build cached data for incremental fetch
                    let cached_titles: HashMap<i32, String> = book
                        .chapters
                        .iter()
                        .filter_map(|ch| {
                            ch.id
                                .strip_prefix("chapter-")
                                .and_then(|n| n.parse::<i32>().ok())
                                .map(|num| (num, ch.title.clone()))
                        })
                        .collect();
                    let cached_data = if cached_titles.is_empty() {
                        None
                    } else {
                        Some((book.chapters.len() as i32, cached_titles))
                    };

                    match api::get_book_details_incremental(source, book.id.clone(), cached_data)
                        .await
                    {
                        Some(details) => {
                            let new_book = Book {
                                id: book.id.clone(),
                                source_id: source.id.clone(),
                                title: details.title,
                                author: details.author,
                                cover_url: details.cover_url,
                                rating: details.rating,
                                status: details.status,
                                chapters_count: details.chapters_count,
                                genres: details.genres,
                                summary: details.summary,
                                in_library: book.in_library,
                                chapters: details
                                    .chapters
                                    .into_iter()
                                    .map(|c| {
                                        // Preserve progress from old book
                                        let old_ch = book.chapters.iter().find(|oc| oc.id == c.id);
                                        Chapter {
                                            id: c.id,
                                            title: c.title,
                                            date: c.date,
                                            progress: old_ch.map(|o| o.progress).unwrap_or(0.0),
                                            last_read: old_ch.map(|o| o.last_read).unwrap_or(0),
                                        }
                                    })
                                    .collect(),
                            };

                            if new_book.in_library {
                                if let Some(ref db) = database {
                                    let _ = db.save_full_book(&new_book);
                                }
                            }

                            let mut s = state.write().unwrap();
                            s.current_book = Some(new_book.clone());
                            if let Some(lib_book) = s.library_books.iter_mut().find(|b| {
                                b.id == new_book.id && b.source_id == new_book.source_id
                            }) {
                                *lib_book = new_book;
                            }
                            s.is_loading = false;
                        }
                        None => {
                            let mut s = state.write().unwrap();
                            s.error_message = Some("Failed to refresh book".to_string());
                            s.is_loading = false;
                        }
                    }
                }
            }
        }

        Command::RefreshLibraryBooks => {
            let (books_to_refresh, sources) = {
                let s = state.read().unwrap();
                let books: Vec<_> = s
                    .library_books
                    .iter()
                    .map(|b| {
                        let cached_titles: HashMap<i32, String> = b
                            .chapters
                            .iter()
                            .filter_map(|ch| {
                                ch.id
                                    .strip_prefix("chapter-")
                                    .and_then(|n| n.parse::<i32>().ok())
                                    .map(|num| (num, ch.title.clone()))
                            })
                            .collect();
                        (b.clone(), cached_titles)
                    })
                    .collect();
                (books, s.sources.clone())
            };

            if books_to_refresh.is_empty() {
                return;
            }

            {
                let mut s = state.write().unwrap();
                s.is_loading = true;
            }
            ctx.request_repaint();

            for (book, cached_titles) in books_to_refresh {
                if let Some(source) = sources.iter().find(|s| s.id == book.source_id) {
                    let cached_data = if cached_titles.is_empty() {
                        None
                    } else {
                        Some((book.chapters.len() as i32, cached_titles))
                    };

                    if let Some(details) =
                        api::get_book_details_incremental(source, book.id.clone(), cached_data)
                            .await
                    {
                        let new_book = Book {
                            id: book.id.clone(),
                            source_id: source.id.clone(),
                            title: details.title,
                            author: details.author,
                            cover_url: details.cover_url,
                            rating: details.rating,
                            status: details.status,
                            chapters_count: details.chapters_count,
                            genres: details.genres,
                            summary: details.summary,
                            in_library: true,
                            chapters: details
                                .chapters
                                .into_iter()
                                .map(|c| {
                                    let old_ch = book.chapters.iter().find(|oc| oc.id == c.id);
                                    Chapter {
                                        id: c.id,
                                        title: c.title,
                                        date: c.date,
                                        progress: old_ch.map(|o| o.progress).unwrap_or(0.0),
                                        last_read: old_ch.map(|o| o.last_read).unwrap_or(0),
                                    }
                                })
                                .collect(),
                        };

                        if let Some(ref db) = database {
                            let _ = db.save_full_book(&new_book);
                        }

                        let mut s = state.write().unwrap();
                        if let Some(lib_book) = s.library_books.iter_mut().find(|b| {
                            b.id == new_book.id && b.source_id == new_book.source_id
                        }) {
                            *lib_book = new_book;
                        }
                    }

                    // Repaint after each book to show progress
                    ctx.request_repaint();
                }
            }

            {
                let mut s = state.write().unwrap();
                s.is_loading = false;
            }
        }

        Command::Search { query } => {
            let source = {
                let s = state.read().unwrap();
                s.selected_source.clone()
            };

            if let Some(source) = source {
                {
                    let mut s = state.write().unwrap();
                    s.is_searching = true;
                    s.error_message = None;
                }
                ctx.request_repaint();

                match api::search_books(&source, &query).await {
                    Some(results) => {
                        let mut s = state.write().unwrap();
                        s.search_results = results;
                        s.is_searching = false;
                    }
                    None => {
                        let mut s = state.write().unwrap();
                        s.error_message = Some("Search failed".to_string());
                        s.is_searching = false;
                    }
                }
            }
        }

        Command::GlobalSearch { query } => {
            let sources = {
                let s = state.read().unwrap();
                s.sources.clone()
            };

            {
                let mut s = state.write().unwrap();
                s.is_global_searching = true;
                s.global_search_streaming = StreamingState::Loading { items_loaded: 0 };
                s.global_search_results.clear();
                s.error_message = None;
            }
            ctx.request_repaint();

            // Create channel for streaming results
            let (results_tx, results_rx) = channel::<Vec<book_core::SearchResult>>();
            let state_clone = Arc::clone(state);
            let ctx_clone = ctx.clone();

            // Spawn a thread to receive streaming results and update state
            let receiver_handle = std::thread::spawn(move || {
                let mut sources_done = 0;
                while let Ok(results) = results_rx.recv() {
                    // Append results to state immediately
                    {
                        let mut s = state_clone.write().unwrap();
                        s.global_search_results.extend(results);
                        sources_done += 1;
                        s.global_search_streaming = StreamingState::Loading { items_loaded: sources_done };
                    }
                    // Request repaint so UI shows the new results
                    ctx_clone.request_repaint();
                }
            });

            // Start streaming search (sends results as each source completes)
            api::search_all_sources_streaming(&sources, &query, results_tx).await;

            // Wait for receiver to finish processing all results
            let _ = receiver_handle.join();

            // Update final state
            {
                let mut s = state.write().unwrap();
                s.is_global_searching = false;
                s.global_search_streaming = StreamingState::Done;
            }
        }

        Command::AddToLibrary { mut book } => {
            book.in_library = true;
            if let Some(ref db) = database {
                let _ = db.save_full_book(&book);
            }

            let mut s = state.write().unwrap();
            if !s
                .library_books
                .iter()
                .any(|b| b.id == book.id && b.source_id == book.source_id)
            {
                s.library_books.push(book.clone());
            }
            if let Some(ref mut curr) = s.current_book {
                if curr.id == book.id && curr.source_id == book.source_id {
                    curr.in_library = true;
                }
            }
        }

        Command::RemoveFromLibrary { book_id, source_id } => {
            if let Some(ref db) = database {
                let _ = db.remove_from_library(&book_id, &source_id);
            }

            let mut s = state.write().unwrap();
            s.library_books
                .retain(|b| !(b.id == book_id && b.source_id == source_id));
            if let Some(ref mut curr) = s.current_book {
                if curr.id == book_id && curr.source_id == source_id {
                    curr.in_library = false;
                }
            }
        }

        Command::ChangeSource { source } => {
            let mut s = state.write().unwrap();
            s.selected_source = Some(source);
            s.discover_sections.clear();
            s.search_results.clear();
            s.discover_streaming = StreamingState::Idle;
        }

        Command::SaveSource { source } => {
            // Save to database
            if let Some(ref db) = database {
                let _ = db.save_source_with_config(&source);
            }

            // Update state
            let mut s = state.write().unwrap();
            // Check if source already exists (update) or is new (add)
            if let Some(existing) = s.sources.iter_mut().find(|s| s.id == source.id) {
                *existing = source.clone();
            } else {
                s.sources.push(source.clone());
            }

            // If no source is selected, select this one
            if s.selected_source.is_none() {
                s.selected_source = Some(source);
            }
        }

        Command::DeleteSource { source_id } => {
            // Don't delete if it's the only source
            {
                let s = state.read().unwrap();
                if s.sources.len() <= 1 {
                    return;
                }
            }

            // Delete from database
            if let Some(ref db) = database {
                let _ = db.delete_source(&source_id);
            }

            // Update state
            let mut s = state.write().unwrap();
            s.sources.retain(|s| s.id != source_id);

            // If deleted source was selected, switch to another
            if s.selected_source.as_ref().map(|src| &src.id) == Some(&source_id) {
                s.selected_source = s.sources.first().cloned();
                s.discover_sections.clear();
                s.search_results.clear();
                s.discover_streaming = StreamingState::Idle;
            }
        }

        Command::CacheCover {
            source_id,
            book_id,
            cover_url,
        } => {
            // Run cover caching in a separate thread to not block
            std::thread::spawn(move || {
                cache_cover_sync(&source_id, &book_id, &cover_url);
            });
        }

        Command::UpdateChapterProgress {
            book_id,
            chapter_id,
            progress,
        } => {
            let source_id = {
                let s = state.read().unwrap();
                s.selected_source.as_ref().map(|src| src.id.clone())
                    .or_else(|| s.current_book.as_ref().map(|b| b.source_id.clone()))
            };

            if let Some(source_id) = source_id {
                // Update state
                {
                    let mut s = state.write().unwrap();

                    // Update current book
                    if let Some(ref mut book) = s.current_book {
                        if book.id == book_id {
                            if let Some(ch) = book.chapters.iter_mut().find(|c| c.id == chapter_id) {
                                ch.progress = progress;
                                ch.last_read = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0);
                            }
                        }
                    }

                    // Update library book
                    if let Some(lib_book) = s.library_books.iter_mut()
                        .find(|b| b.id == book_id && b.source_id == source_id)
                    {
                        if let Some(ch) = lib_book.chapters.iter_mut().find(|c| c.id == chapter_id) {
                            ch.progress = progress;
                            ch.last_read = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0);
                        }
                        // Save to database
                        if let Some(ref db) = database {
                            let _ = db.save_full_book(lib_book);
                        }
                    }
                }
            }
        }

        Command::SaveBookToCache { book } => {
            // Save a book with its chapters to the database cache
            if let Some(ref db) = database {
                let _ = db.save_full_book(&book);
                println!("[LOGIC] Cached book '{}' with {} chapters", book.title, book.chapters.len());
            }
        }
    }
}
