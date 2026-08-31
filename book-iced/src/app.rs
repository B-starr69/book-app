use iced::widget::{Image, button, column, container, row, scrollable, text, text_input, rich_text, span};
use iced::{Alignment, Element, Length, Task};
use std::sync::Arc;

use book_core::database::Database;
use book_core::models::Book;
use book_core::{HomeSection, SourceWithConfig, importer};

use crate::book_render::RenderIcedBook;
use crate::helpers::{bold_text, space_fill_x, space_fill_y, space_y};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Library,
    Discover,
    Search,
    Settings,
}

#[derive(Clone)]
pub enum Message {
    DatabaseInitialized(Arc<Database>, Vec<SourceWithConfig>, Vec<Book>),
    TabSelected(Tab),
    LoadLibrary,
    LibraryLoaded(Vec<Book>),
    LoadDiscoverData,
    DiscoverDataLoaded(Result<Vec<HomeSection>, String>),
    GithubUrlChanged(String),
    ImportSources,
    SourcesImported(Result<Vec<String>, String>),
    SourcesLoaded(Vec<SourceWithConfig>),
    SourceSelected(String),
    SearchKeywordChanged(String),
    TriggerSearch,
    SearchResultsLoaded(Result<Vec<book_core::SearchResult>, String>),
    BookFetched(Result<Book, String>),
    BookFetchedAndView(Result<Book, String>),
    LoadBookDetails(String),
    ViewBook(Option<Book>),
    ChaptersPageFetched(String, String, i32, Result<Vec<book_core::models::Chapter>, String>),
    FetchNextDiscoverBook,
    DiscoverBookFetched(Result<Book, String>),
    FreshCachedBooksLoaded(Result<Vec<Book>, String>),
    ToggleLibraryStatus(String, String, bool),
    LibraryStatusUpdated(String, String, bool, Result<(), String>),
    LoadChapter(String, String, bool),
    ChapterLoaded(String, String, String, Result<String, String>),
    CloseReader,
    DatabaseUpdated,
    ForceSyncBook(String),
    ToggleBookMenu,
    ReaderScrolled(iced::widget::scrollable::Viewport),
    ImageDownloaded(Result<std::path::PathBuf, String>),
}

pub struct MyApp {
    pub active_tab: Tab,
    pub sources: Vec<SourceWithConfig>,
    pub discover_sections: Vec<HomeSection>,
    pub books: Vec<Book>,
    pub is_loading_library: bool,
    pub is_loading_discover: bool,
    pub database: Option<Arc<Database>>,
    pub github_url: String,
    pub import_status: Option<Result<String, String>>,
    pub is_importing: bool,
    pub selected_source_id: Option<String>,
    pub search_keyword: String,
    pub search_results: Option<Vec<book_core::SearchResult>>,
    pub is_searching: bool,
    pub discover_error: Option<String>,
    pub search_error: Option<String>,
    pub viewed_book: Option<Book>,
    pub discover_fetch_queue: Vec<String>,
    pub active_chapter: Option<(book_core::models::Chapter, Vec<crate::html_parser::ReaderBlock>)>,
    pub is_loading_chapter: bool,
    pub chapter_load_error: Option<String>,
    pub loading_text: Option<String>,
    pub show_book_menu: bool,
}

impl MyApp {
    pub fn new() -> (Self, Task<Message>) {
        let app = Self {
            database: None,
            sources: Vec::new(),
            active_tab: Tab::Library,
            books: Vec::new(),
            is_loading_library: false,
            is_loading_discover: false,
            discover_sections: Vec::new(),
            github_url: String::new(),
            import_status: None,
            is_importing: false,
            selected_source_id: None,
            search_keyword: String::new(),
            search_results: None,
            is_searching: false,
            discover_error: None,
            search_error: None,
            viewed_book: None,
            discover_fetch_queue: Vec::new(),
            active_chapter: None,
            is_loading_chapter: false,
            chapter_load_error: None,
            loading_text: None,
            show_book_menu: false,
        };

        let task = Task::perform(
            async {
                let db = Database::open_local()
                    .await
                    .expect("Failed to open local database");
                let sources = db.get_sources().await.unwrap_or_default();
                let library_books = db.get_library_books().await.unwrap_or_default();
                (Arc::new(db), sources, library_books)
            },
            |(db, sources, books)| Message::DatabaseInitialized(db, sources, books),
        );

        (app, task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DatabaseInitialized(db, sources, books) => {
                self.database = Some(db);
                self.sources = sources;
                self.books = books;
                self.selected_source_id = self.sources.first().map(|s| s.source.id.clone());
                Task::none()
            }
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                self.viewed_book = None; // Reset detail page when switching tabs
                match tab {
                    Tab::Library if self.books.is_empty() => Task::done(Message::LoadLibrary),
                    Tab::Discover if self.discover_sections.is_empty() => {
                        Task::done(Message::LoadDiscoverData)
                    }
                    _ => Task::none(),
                }
            }
            Message::LoadLibrary => {
                if let Some(database) = &self.database {
                    let database = Arc::clone(database);
                    self.is_loading_library = true;
                    Task::perform(
                        async move { database.get_library_books().await.unwrap_or_default() },
                        Message::LibraryLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::LibraryLoaded(books) => {
                self.is_loading_library = false;
                for book in books {
                    if let Some(existing) = self.books.iter_mut().find(|b| b.id() == book.id() && b.source_id() == book.source_id()) {
                        existing.base_mut().in_library = book.base().in_library;
                    } else {
                        self.books.push(book);
                    }
                }
                Task::none()
            }
            Message::LoadDiscoverData => {
                self.is_loading_discover = true;
                self.discover_error = None;
                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                if let Some(source) = source
                    && (source.source.id != "local")
                {
                    Task::perform(
                        async move { book_core::api::get_discover_page(&source).await },
                        Message::DiscoverDataLoaded,
                    )
                } else {
                    self.is_loading_discover = false;
                    Task::none()
                }
            }
            Message::DiscoverDataLoaded(result) => {
                self.is_loading_discover = false;
                match result {
                    Ok(sections) => {
                        self.discover_sections = sections;
                        self.discover_error = None;
                    }
                    Err(err) => {
                        self.discover_sections = Vec::new();
                        self.discover_error = Some(err);
                        return Task::none();
                    }
                }

                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                let source = match source {
                    Some(s) => s,
                    None => return Task::none(),
                };

                let ids: Vec<String> = self
                    .discover_sections
                    .iter()
                    .flat_map(|section| section.books.clone())
                    .collect();

                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(),
                };
                let source_id = source.source.id.clone();

                // Fetch cached, non-stale books from local database instantly in the background
                Task::perform(
                    async move {
                        book_core::api::get_fresh_cached_books(&*db, &source_id, ids).await
                    },
                    Message::FreshCachedBooksLoaded,
                )
            }
            Message::FreshCachedBooksLoaded(res) => {
                match res {
                    Ok(cached_books) => {
                        for book in cached_books {
                            if !self.books.iter().any(|b| b.id() == book.id() && b.source_id() == book.source_id()) {
                                self.books.push(book);
                            }
                        }
                    }
                    Err(err) => {
                        println!("Failed to load cached discover books: {err}");
                    }
                }

                let ids: Vec<String> = self
                    .discover_sections
                    .iter()
                    .flat_map(|section| section.books.clone())
                    .collect();

                let existing_ids: std::collections::HashSet<_> =
                    self.books.iter().map(|b| b.id().to_string()).collect();

                let ids_to_fetch: Vec<String> = ids
                    .into_iter()
                    .filter(|id| !existing_ids.contains(id))
                    .collect();

                self.discover_fetch_queue = ids_to_fetch;

                // Start sequential background fetching of only the remaining uncached/stale books
                Task::perform(async {}, |_| Message::FetchNextDiscoverBook)
            }
            Message::FetchNextDiscoverBook => {
                if self.discover_fetch_queue.is_empty() {
                    return Task::none();
                }

                let id = self.discover_fetch_queue.remove(0);
                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(),
                };
                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                if let Some(source) = source {
                    Task::perform(
                        async move {
                            // Sleep 300ms between requests to be polite and avoid rate limits
                            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                            let res = book_core::api::get_book(&*db, &source, &id, false, false).await;
                            Message::DiscoverBookFetched(res)
                        },
                        |msg| msg,
                    )
                } else {
                    Task::none()
                }
            }
            Message::DiscoverBookFetched(result) => {
                match result {
                    Ok(book) => {
                        if !self.books.iter().any(|b| b.id() == book.id()) {
                            self.books.push(book);
                        }
                    }
                    Err(err) => {
                        println!("Failed to fetch discover book: {err}");
                    }
                }
                // Trigger fetch of next book in discover queue
                Task::perform(async {}, |_| Message::FetchNextDiscoverBook)
            }
            Message::BookFetched(result) => {
                match result {
                    Ok(book) => {
                        self.books.push(book);
                    }
                    Err(err) => {
                        println!("Failed to fetch book: {err}");
                    }
                }
                Task::none()
            }
            Message::BookFetchedAndView(result) => {
                self.show_book_menu = false;
                self.is_loading_chapter = false;
                self.loading_text = None;
                match result {
                    Ok(book) => {
                        if !self.books.iter().any(|b| b.id() == book.id()) {
                            self.books.push(book.clone());
                        }
                        self.viewed_book = Some(book.clone());

                        if let Book::WebNovel(ref webnovel) = book {
                            let total_count = webnovel.chapters_count as usize;
                            let current_count = webnovel.chapters.len();
                            if current_count < total_count {
                                let source = self
                                    .sources
                                    .iter()
                                    .find(|s| s.source.id == webnovel.base.source_id)
                                    .cloned();
                                if let Some(source) = source {
                                    let db = self.database.as_ref().unwrap().clone();
                                    let book_id = webnovel.base.id.clone();
                                    let page_size = 100; // Default chapters per page
                                    let next_page = (current_count / page_size) as i32 + 1;
                                    return Task::perform(
                                        async move {
                                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                            let res = book_core::api::sync_chapters_page(&*db, &source, &book_id, next_page).await;
                                            Message::ChaptersPageFetched(book_id, source.source.id, next_page, res)
                                        },
                                        |msg| msg,
                                    );
                                }
                            }
                        }
                    }
                    Err(err) => {
                        println!("Failed to fetch book details: {err}");
                    }
                }
                Task::none()
            }
            Message::ChaptersPageFetched(book_id, source_id, page_fetched, res) => {
                match res {
                    Ok(new_chapters) => {
                        if let Some(Book::WebNovel(ref mut webnovel)) = self.viewed_book {
                            if webnovel.base.id == book_id && webnovel.base.source_id == source_id {
                                for chap in new_chapters {
                                    if !webnovel.chapters.iter().any(|c| c.id == chap.id) {
                                        webnovel.chapters.push(chap);
                                    }
                                }

                                // Check if we need to load even MORE pages!
                                let total_count = webnovel.chapters_count as usize;
                                let current_count = webnovel.chapters.len();
                                if current_count < total_count {
                                    let source = self
                                        .sources
                                        .iter()
                                        .find(|s| s.source.id == source_id)
                                        .cloned();
                                    if let Some(source) = source {
                                        let db = self.database.as_ref().unwrap().clone();
                                        let next_page = page_fetched + 1;
                                        return Task::perform(
                                            async move {
                                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                                let res = book_core::api::sync_chapters_page(&*db, &source, &book_id, next_page).await;
                                                Message::ChaptersPageFetched(book_id, source_id, next_page, res)
                                            },
                                            |msg| msg,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to fetch chapters page {}: {}", page_fetched, e);
                    }
                }
                Task::none()
            }
            Message::LoadBookDetails(id) => {
                self.show_book_menu = false;
                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(),
                };
                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                if let Some(source) = source {
                    self.is_loading_chapter = true;
                    self.loading_text = Some("Loading book details...".to_string());
                    Task::perform(
                        async move { book_core::api::get_book(&*db, &source, &id, true, false).await },
                        Message::BookFetchedAndView,
                    )
                } else {
                    Task::none()
                }
            }
            Message::ViewBook(book_opt) => {
                self.show_book_menu = false;
                self.viewed_book = book_opt.clone();
                if let Some(book) = book_opt {
                    let db = match &self.database {
                        Some(db) => Arc::clone(db),
                        None => return Task::none(),
                    };
                    let source = self
                        .sources
                        .iter()
                        .find(|s| s.source.id == book.source_id())
                        .cloned();

                    if let Some(source) = source {
                        let id = book.id().to_string();
                        // Sync/update book details and check for missing chapters in the background
                        return Task::perform(
                            async move { book_core::api::get_book(&*db, &source, &id, true, false).await },
                            Message::BookFetchedAndView,
                        );
                    }
                }
                Task::none()
            }
            Message::ToggleLibraryStatus(book_id, source_id, in_library) => {
                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(),
                };

                // Update in-memory viewed_book immediately for responsive feedback
                if let Some(ref mut book) = self.viewed_book {
                    if book.id() == book_id && book.source_id() == source_id {
                        book.base_mut().in_library = in_library;
                    }
                }
                if let Some(book) = self.books.iter_mut().find(|b| b.id() == book_id && b.source_id() == source_id) {
                    book.base_mut().in_library = in_library;
                }

                let b_id = book_id.clone();
                let s_id = source_id.clone();

                Task::perform(
                    async move {
                        let res = db.set_in_library(&b_id, &s_id, in_library).await
                            .map_err(|e| e.to_string());
                        Message::LibraryStatusUpdated(b_id, s_id, in_library, res)
                    },
                    |msg| msg,
                )
            }
            Message::LibraryStatusUpdated(book_id, source_id, in_library, res) => {
                if let Err(e) = res {
                    eprintln!("Failed to update library status: {e}");
                    // Revert on failure
                    if let Some(ref mut book) = self.viewed_book {
                        if book.id() == book_id && book.source_id() == source_id {
                            book.base_mut().in_library = !in_library;
                        }
                    }
                    if let Some(book) = self.books.iter_mut().find(|b| b.id() == book_id && b.source_id() == source_id) {
                        book.base_mut().in_library = !in_library;
                    }
                } else {
                    // Trigger a reload of library books
                    if let Some(db) = &self.database {
                        let db = Arc::clone(db);
                        return Task::perform(
                            async move { db.get_library_books().await.unwrap_or_default() },
                            Message::LibraryLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::LoadChapter(chapter_id, _chapter_title, force_refresh) => {
                self.show_book_menu = false;
                let book = match &self.viewed_book {
                    Some(book) => book,
                    None => return Task::none(),
                };
                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(),
                };
                let source = self
                    .sources
                    .iter()
                    .find(|s| s.source.id == book.source_id())
                    .cloned();

                if let Some(source) = source {
                    let mut save_task = Task::none();
                    if let Some((old_chapter, _)) = &self.active_chapter {
                        let old_b_id = book.id().to_string();
                        let old_s_id = book.source_id().to_string();
                        let old_c_id = old_chapter.id.clone();
                        let old_progress = old_chapter.progress;
                        let old_db = Arc::clone(&db);
                        save_task = Task::perform(
                            async move {
                                let _ = old_db.update_chapter_progress(&old_b_id, &old_s_id, &old_c_id, old_progress).await;
                            },
                            |_| Message::DatabaseUpdated,
                        );
                    }

                    self.is_loading_chapter = true;
                    self.chapter_load_error = None;
                    if force_refresh {
                        self.loading_text = Some("Forcing fresh sync of chapter content...".to_string());
                    } else {
                        self.loading_text = Some("Loading chapter content...".to_string());
                    }
                    let book_id = book.id().to_string();
                    let chap_id = chapter_id.clone();

                    let b_id = book_id.clone();
                    let s_id = source.source.id.clone();
                    let c_id = chap_id.clone();
                    let load_task = Task::perform(
                        async move {
                            book_core::api::get_chapter_content(&*db, &source, &book_id, &chap_id, force_refresh).await
                        },
                        move |res| {
                            Message::ChapterLoaded(b_id.clone(), s_id.clone(), c_id.clone(), res)
                        },
                    );
                    Task::batch(vec![save_task, load_task])
                } else {
                    Task::none()
                }
            }
            Message::ForceSyncBook(id) => {
                self.show_book_menu = false;
                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(),
                };
                let book = match &self.viewed_book {
                    Some(b) => b,
                    None => return Task::none(),
                };
                let source = self
                    .sources
                    .iter()
                    .find(|s| s.source.id == book.source_id())
                    .cloned();

                if let Some(source) = source {
                    self.is_loading_chapter = true;
                    self.loading_text = Some("Syncing book details & chapters list from web...".to_string());
                    Task::perform(
                        async move { book_core::api::get_book(&*db, &source, &id, true, true).await },
                        Message::BookFetchedAndView,
                    )
                } else {
                    Task::none()
                }
            }
            Message::ToggleBookMenu => {
                self.show_book_menu = !self.show_book_menu;
                Task::none()
            }
            Message::ReaderScrolled(viewport) => {
                if let Some((ref mut chapter, _)) = self.active_chapter {
                    chapter.progress = viewport.relative_offset().y;
                }
                Task::none()
            }
            Message::ChapterLoaded(book_id, source_id, chapter_id, result) => {
                self.is_loading_chapter = false;
                match result {
                    Ok(content) => {
                        if let Some(book) = &self.viewed_book {
                            if book.id() == book_id && book.source_id() == source_id {
                                let chapter_opt = match book {
                                    Book::WebNovel(wn) => wn.chapters.iter().find(|c| c.id == chapter_id).cloned(),
                                    _ => None,
                                };

                                if let Some(chapter) = chapter_opt {
                                    let blocks = crate::html_parser::parse_html(&content);
                                    self.active_chapter = Some((chapter.clone(), blocks.clone()));
                                    self.chapter_load_error = None;

                                    // Mark chapter as read in DB and locally
                                    let db = self.database.as_ref().unwrap().clone();
                                    let b_id = book_id.clone();
                                    let s_id = source_id.clone();
                                    let c_id = chapter_id.clone();

                                    if let Some(Book::WebNovel(ref mut webnovel)) = self.viewed_book {
                                        if let Some(chap) = webnovel.chapters.iter_mut().find(|c| c.id == c_id) {
                                            chap.last_read = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs() as i64;
                                        }
                                    }

                                    let initial_progress = chapter.progress;
                                    let task_b_id = b_id.clone();
                                    let task_s_id = s_id.clone();
                                    let task_c_id = c_id.clone();
                                    let db_task = Task::perform(
                                        async move {
                                            let _ = db.update_chapter_progress(&task_b_id, &task_s_id, &task_c_id, initial_progress).await;
                                        },
                                        |_| Message::DatabaseUpdated,
                                    );

                                    let scroll_task = if initial_progress > 0.0 {
                                        iced::widget::operation::snap_to(
                                            iced::widget::Id::new("reader_scroll"),
                                            iced::widget::operation::RelativeOffset {
                                                x: 0.0,
                                                y: initial_progress,
                                            },
                                        )
                                    } else {
                                        Task::none()
                                    };

                                    let mut tasks = vec![db_task, scroll_task];

                                    for block in &blocks {
                                        if let crate::html_parser::ReaderBlock::Image(url) = block {
                                            let local_path = book_core::storage::webnovel_image_path(&s_id, &b_id, url);
                                            if !local_path.exists() {
                                                let ref_s_id = s_id.clone();
                                                let ref_b_id = b_id.clone();
                                                let img_url = url.clone();
                                                tasks.push(Task::perform(
                                                    async move {
                                                        book_core::storage::download_image_if_needed(&ref_s_id, &ref_b_id, &img_url).await
                                                    },
                                                    Message::ImageDownloaded,
                                                ));
                                            }
                                        }
                                    }

                                    return Task::batch(tasks);
                                }
                            }
                        }
                    }
                    Err(err) => {
                        self.chapter_load_error = Some(err);
                    }
                }
                Task::none()
            }
            Message::ImageDownloaded(res) => {
                if let Err(e) = res {
                    eprintln!("Error downloading chapter image: {}", e);
                }
                Task::none()
            }
            Message::CloseReader => {
                self.show_book_menu = false;
                if let (Some((chapter, _)), Some(book)) = (&self.active_chapter, &self.viewed_book) {
                    let db = self.database.as_ref().unwrap().clone();
                    let b_id = book.id().to_string();
                    let s_id = book.source_id().to_string();
                    let c_id = chapter.id.clone();
                    let progress = chapter.progress;

                    self.active_chapter = None;

                    let ref_b_id = b_id.clone();
                    let ref_db = db.clone();

                    let source_opt = self.sources.iter().find(|s| s.source.id == s_id).cloned();

                    if let Some(source) = source_opt {
                        return Task::perform(
                            async move {
                                let _ = db.update_chapter_progress(&b_id, &s_id, &c_id, progress).await;
                                book_core::api::get_book(&*ref_db, &source, &ref_b_id, false, false).await
                            },
                            |res| match res {
                                Ok(refreshed_book) => Message::ViewBook(Some(refreshed_book)),
                                Err(_) => Message::DatabaseUpdated,
                            },
                        );
                    }
                }
                self.active_chapter = None;
                Task::none()
            }
            Message::DatabaseUpdated => {
                Task::none()
            }
            Message::GithubUrlChanged(url) => {
                self.github_url = url;
                Task::none()
            }
            Message::ImportSources => {
                if self.github_url.trim().is_empty() {
                    return Task::none();
                }

                if let Some(database) = &self.database {
                    self.is_importing = true;
                    self.import_status = None;

                    let repo_url = self.github_url.clone();
                    let base_dir = std::env::current_dir().unwrap_or_default();
                    let database = database.clone();

                    Task::perform(
                        async move {
                            let db_client = database;
                            let gitres =
                                importer::import_from_github(&repo_url, &base_dir, &db_client)
                                    .await
                                    .map_err(|e| e.to_string());
                            Message::SourcesImported(gitres)
                        },
                        |msg| msg,
                    )
                } else {
                    Task::none()
                }
            }
            Message::SourcesImported(result) => {
                self.is_importing = false;
                match result {
                    Ok(imported_ids) => {
                        self.import_status = Some(Ok(format!(
                            "Successfully imported {} sources: {}",
                            imported_ids.len(),
                            imported_ids.join(", ")
                        )));
                        self.github_url.clear();

                        if let Some(database) = &self.database {
                            let database = database.clone();
                            Task::perform(
                                async move { database.get_sources().await.unwrap_or_default() },
                                Message::SourcesLoaded,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    Err(err) => {
                        self.import_status = Some(Err(format!("Import failed: {}", err)));
                        Task::none()
                    }
                }
            }
            Message::SourcesLoaded(sources) => {
                self.sources = sources;
                if self.selected_source_id.is_none() {
                    self.selected_source_id = self.sources.first().map(|s| s.source.id.clone());
                }
                Task::none()
            }
            Message::SourceSelected(id) => {
                self.selected_source_id = Some(id);
                self.discover_sections.clear();
                Task::done(Message::LoadDiscoverData)
            }
            Message::SearchKeywordChanged(keyword) => {
                self.search_keyword = keyword;
                Task::none()
            }
            Message::TriggerSearch => {
                if self.search_keyword.trim().is_empty() {
                    return Task::none();
                }
                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                if let Some(source) = source {
                    self.is_searching = true;
                    self.search_results = None;
                    self.search_error = None;
                    let keyword = self.search_keyword.clone();
                    Task::perform(
                        async move { book_core::api::search_books(&source, &keyword, None).await },
                        Message::SearchResultsLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::SearchResultsLoaded(result) => {
                self.is_searching = false;
                match result {
                    Ok(results) => {
                        self.search_results = Some(results);
                        self.search_error = None;
                    }
                    Err(err) => {
                        self.search_results = None;
                        self.search_error = Some(err);
                    }
                }
                Task::none()
            }
        }
    }

    fn nav_button<'a>(&self, label: &'a str, tab: Tab, is_selected: bool) -> Element<'a, Message> {
        let label_text = bold_text(label).size(15).color(if is_selected {
            theme::TEXT_SLATE_50
        } else {
            theme::TEXT_SLATE_300
        });

        let btn = button(container(label_text).padding(6))
            .on_press(Message::TabSelected(tab))
            .width(Length::Fill)
            .style(move |_theme, status| {
                let bg_color = if is_selected {
                    theme::ACCENT_INDIGO_500
                } else if status == button::Status::Hovered {
                    theme::BG_SLATE_700
                } else {
                    iced::Color::TRANSPARENT
                };
                button::Style {
                    background: Some(iced::Background::Color(bg_color)),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        container(btn).padding(8).into()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = container(
            column![
                container(bold_text("novel app").size(20).color(theme::TEXT_SLATE_50))
                    .padding(20)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                self.nav_button(
                    "Library",
                    Tab::Library,
                    self.active_tab == Tab::Library && self.viewed_book.is_none()
                ),
                self.nav_button(
                    "Discover",
                    Tab::Discover,
                    self.active_tab == Tab::Discover && self.viewed_book.is_none()
                ),
                self.nav_button(
                    "Search",
                    Tab::Search,
                    self.active_tab == Tab::Search && self.viewed_book.is_none()
                ),
                self.nav_button(
                    "Sources & Settings",
                    Tab::Settings,
                    self.active_tab == Tab::Settings && self.viewed_book.is_none()
                ),
                space_fill_y(),
                container(text("v0.2.0").size(12).color(theme::TEXT_SLATE_400))
                    .padding(15)
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
            ]
            .spacing(5),
        )
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(theme::BG_SLATE_800)),
            border: iced::Border {
                color: theme::BG_SLATE_700,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

        let main_content = container(if self.is_loading_chapter {
            let label = self.loading_text.as_deref().unwrap_or("Loading content...");
            container(
                column![
                    bold_text(label).size(20).color(theme::TEXT_SLATE_50),
                    space_y(15.0),
                    text("Fetching page from web novel source").size(14).color(theme::TEXT_SLATE_400),
                ]
                .align_x(Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else if let Some((chapter, content)) = &self.active_chapter {
            self.render_reader(chapter, content)
        } else if let Some(book) = &self.viewed_book {
            self.render_book_detail(book)
        } else {
            match self.active_tab {
                Tab::Library => self.render_library(),
                Tab::Discover => self.render_discover(),
                Tab::Search => self.render_search(),
                Tab::Settings => self.render_settings(),
            }
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(25)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(theme::BG_SLATE_900)),
            ..Default::default()
        });

        row![sidebar, main_content].into()
    }

    fn render_book_detail<'a>(&'a self, book: &'a Book) -> Element<'a, Message> {
        let back_btn = button(text("← Back").size(14).color(theme::TEXT_SLATE_50))
            .on_press(Message::ViewBook(None))
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(
                    if status == button::Status::Hovered {
                        theme::ACCENT_INDIGO_600
                    } else {
                        theme::ACCENT_INDIGO_500
                    },
                )),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(10);

        let detail_element = match book {
            Book::Novel(novel) => novel.render_detail(self.show_book_menu),
            Book::WebNovel(wn) => wn.render_detail(self.show_book_menu),
        };

        column![back_btn, space_y(20.0), detail_element]
            .spacing(10)
            .into()
    }

    fn render_library(&self) -> Element<'_, Message> {
        let title_row = row![
            bold_text("My Library").size(26).color(theme::TEXT_SLATE_50),
            space_fill_x(),
            button(text("🔄 Refresh").size(14).color(theme::TEXT_SLATE_50))
                .on_press(Message::LoadLibrary)
                .style(|_, status| button::Style {
                    background: Some(iced::Background::Color(
                        if status == button::Status::Hovered {
                            theme::ACCENT_INDIGO_600
                        } else {
                            theme::ACCENT_INDIGO_500
                        }
                    )),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(10)
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill);

        let content: Element<'_, Message> =
            if self.is_loading_library {
                container(
                    text("Loading library books...")
                        .size(16)
                        .color(theme::TEXT_SLATE_400),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
            } else if self.books.is_empty() {
                container(
                    column![
                        bold_text("Your library is empty")
                            .size(18)
                            .color(theme::TEXT_SLATE_300),
                        space_y(10.0),
                        text("Browse online sources or search to add books here.")
                            .size(14)
                            .color(theme::TEXT_SLATE_400),
                    ]
                    .align_x(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
            } else {
                let books_col = column(self.books.iter().filter(|b| b.base().in_library).map(
                    |book| {
                        let base = book.base();
                        let cover: Element<'_, Message> = if let Some(path) =
                            book_core::storage::get_cover_path(
                                &base.source_id,
                                &base.id,
                                &base.cover_url,
                            ) {
                            if path.exists() {
                                let img: Image =
                                    Image::new(iced::widget::image::Handle::from_path(path));
                                img.width(60).height(80).into()
                            } else {
                                container(text("📖").size(32))
                                    .width(Length::Fixed(60.0))
                                    .height(Length::Fixed(80.0))
                                    .align_x(Alignment::Center)
                                    .align_y(Alignment::Center)
                                    .style(|_| container::Style {
                                        background: Some(iced::Background::Color(
                                            theme::BG_SLATE_700,
                                        )),
                                        border: iced::Border {
                                            radius: 6.0.into(),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    })
                                    .into()
                            }
                        } else {
                            container(text("📖").size(32))
                                .width(Length::Fixed(60.0))
                                .height(Length::Fixed(80.0))
                                .align_x(Alignment::Center)
                                .align_y(Alignment::Center)
                                .style(|_| container::Style {
                                    background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                                    border: iced::Border {
                                        radius: 6.0.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .into()
                        };

                        let book_info = column![
                            bold_text(&base.title).size(16).color(theme::TEXT_SLATE_50),
                            text(&base.author).size(14).color(theme::TEXT_SLATE_400),
                            row![
                                text(format!("⭐ {:.1}", base.rating))
                                    .size(12)
                                    .color(theme::TEXT_SLATE_300),
                                text(&base.status).size(12).color(theme::TEXT_SLATE_400),
                            ]
                            .spacing(10),
                        ]
                        .spacing(4);

                        let card_row = row![cover, book_info]
                            .spacing(15)
                            .align_y(Alignment::Center);

                        button(card_row)
                            .on_press(Message::ViewBook(Some(book.clone())))
                            .padding(12)
                            .width(Length::Fill)
                            .style(|_, status| button::Style {
                                background: Some(iced::Background::Color(
                                    if status == button::Status::Hovered {
                                        theme::BG_SLATE_700
                                    } else {
                                        theme::BG_SLATE_800
                                    },
                                )),
                                border: iced::Border {
                                    color: theme::BG_SLATE_700,
                                    width: 1.0,
                                    radius: 8.0.into(),
                                },
                                text_color: theme::TEXT_SLATE_50,
                                ..Default::default()
                            })
                            .into()
                    },
                ))
                .spacing(10);

                scrollable(books_col).into()
            };

        column![title_row, space_y(20.0), content]
            .spacing(10)
            .into()
    }

    fn render_discover(&self) -> Element<'_, Message> {
        let title_row = row![
            bold_text("Discover").size(26).color(theme::TEXT_SLATE_50),
            space_fill_x(),
            button(text("🔄 Refresh").size(14).color(theme::TEXT_SLATE_50))
                .on_press(Message::LoadDiscoverData)
                .style(|_, status| button::Style {
                    background: Some(iced::Background::Color(
                        if status == button::Status::Hovered {
                            theme::ACCENT_INDIGO_600
                        } else {
                            theme::ACCENT_INDIGO_500
                        }
                    )),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(10)
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill);

        let source_buttons =
            row(self
                .sources
                .iter()
                .filter(|s| s.source.id != "local")
                .map(|source| {
                    let is_selected = Some(&source.source.id) == self.selected_source_id.as_ref();
                    let label = text(&source.source.name).size(13).color(if is_selected {
                        theme::TEXT_SLATE_50
                    } else {
                        theme::TEXT_SLATE_300
                    });

                    button(container(label).padding(4))
                        .on_press(Message::SourceSelected(source.source.id.clone()))
                        .style(move |_, status| {
                            let bg_color = if is_selected {
                                theme::ACCENT_INDIGO_500
                            } else if status == button::Status::Hovered {
                                theme::BG_SLATE_700
                            } else {
                                theme::BG_SLATE_850
                            };
                            button::Style {
                                background: Some(iced::Background::Color(bg_color)),
                                border: iced::Border {
                                    color: theme::BG_SLATE_700,
                                    width: 1.0,
                                    radius: 6.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                        .into()
                }))
            .spacing(10);

        let source_selector = column![
            text("Select Source:").size(14).color(theme::TEXT_SLATE_400),
            space_y(5.0),
            source_buttons
        ];

        let content: Element<'_, Message> = if self.is_loading_discover {
            container(
                text("Loading discover content...")
                    .size(16)
                    .color(theme::TEXT_SLATE_400),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else if let Some(err) = &self.discover_error {
            container(
                column![
                    bold_text("Failed to load discover content")
                        .size(18)
                        .color(theme::ERROR_RED),
                    space_y(10.0),
                    text(err).size(14).color(theme::TEXT_SLATE_300),
                ]
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else if self.discover_sections.is_empty() {
            container(
                column![
                    bold_text("No discover data found")
                        .size(18)
                        .color(theme::TEXT_SLATE_300),
                    space_y(10.0),
                    text("Check your internet connection or verify the source configuration.")
                        .size(14)
                        .color(theme::TEXT_SLATE_400),
                ]
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else {
            let sections_col = column(self.discover_sections.iter().map(|section| {
                let books_row = row(section.books.iter().map(|book_name| {
                    let book = self.books.iter().find(|f| f.id() == book_name);
                    match book {
                        Some(book) => {
                            let card_content = match book {
                                Book::Novel(novel) => novel.render_card(),
                                Book::WebNovel(wn) => wn.render_card(),
                            };
                            button(card_content)
                                .on_press(Message::ViewBook(Some(book.clone())))
                                .style(|_, status| button::Style {
                                    background: Some(iced::Background::Color(
                                        if status == button::Status::Hovered {
                                            theme::BG_SLATE_700
                                        } else {
                                            theme::BG_SLATE_800
                                        },
                                    )),
                                    border: iced::Border {
                                        color: theme::BG_SLATE_700,
                                        width: 1.0,
                                        radius: 6.0.into(),
                                    },
                                    ..Default::default()
                                })
                                .into()
                        }
                        None => container(
                            column![
                                container(text("📖").size(24))
                                    .width(Length::Fixed(40.0))
                                    .height(Length::Fixed(40.0))
                                    .align_x(Alignment::Center)
                                    .align_y(Alignment::Center)
                                    .style(|_| container::Style {
                                        background: Some(iced::Background::Color(
                                            theme::BG_SLATE_700
                                        )),
                                        border: iced::Border {
                                            radius: 4.0.into(),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    }),
                                space_y(5.0),
                                text(book_name)
                                    .size(12)
                                    .color(theme::TEXT_SLATE_300)
                                    .width(Length::Fixed(120.0)),
                            ]
                            .align_x(Alignment::Center),
                        )
                        .padding(10)
                        .width(Length::Fixed(140.0))
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                            border: iced::Border {
                                color: theme::BG_SLATE_700,
                                width: 1.0,
                                radius: 6.0.into(),
                            },
                            ..Default::default()
                        })
                        .into(),
                    }
                }))
                .spacing(10);

                let horiz_direction = scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::default(),
                );
                let horizontal_scroll = scrollable(books_row).direction(horiz_direction);

                column![
                    bold_text(&section.title)
                        .size(18)
                        .color(theme::TEXT_SLATE_300),
                    space_y(10.0),
                    horizontal_scroll,
                    space_y(15.0),
                ]
                .into()
            }))
            .spacing(10);

            scrollable(sections_col).into()
        };

        column![
            title_row,
            space_y(10.0),
            source_selector,
            space_y(20.0),
            content
        ]
        .spacing(10)
        .into()
    }

    fn render_search(&self) -> Element<'_, Message> {
        let search_input = text_input("Search for books...", &self.search_keyword)
            .on_input(Message::SearchKeywordChanged)
            .on_submit(Message::TriggerSearch)
            .padding(12)
            .style(|_, status| text_input::Style {
                background: iced::Background::Color(theme::BG_SLATE_800),
                border: iced::Border {
                    color: if matches!(status, text_input::Status::Focused { .. }) {
                        theme::ACCENT_INDIGO_500
                    } else {
                        theme::BG_SLATE_700
                    },
                    width: 1.0,
                    radius: 6.0.into(),
                },
                value: theme::TEXT_SLATE_50,
                placeholder: theme::TEXT_SLATE_400,
                selection: theme::ACCENT_INDIGO_500,
                icon: theme::TEXT_SLATE_400,
            });

        let search_button = button(text("Search").color(theme::TEXT_SLATE_50))
            .on_press(Message::TriggerSearch)
            .padding(12)
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(
                    if status == button::Status::Hovered {
                        theme::ACCENT_INDIGO_600
                    } else {
                        theme::ACCENT_INDIGO_500
                    },
                )),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let search_bar = row![search_input, search_button]
            .spacing(10)
            .width(Length::Fill);

        let results_content: Element<'_, Message> = if self.is_searching {
            container(text("Searching...").size(16).color(theme::TEXT_SLATE_400))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        } else if let Some(err) = &self.search_error {
            container(
                column![
                    bold_text("Search failed").size(18).color(theme::ERROR_RED),
                    space_y(10.0),
                    text(err).size(14).color(theme::TEXT_SLATE_300),
                ]
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        } else if let Some(results) = &self.search_results {
            if results.is_empty() {
                container(
                    text("No results found.")
                        .size(16)
                        .color(theme::TEXT_SLATE_400),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
            } else {
                let results_col = column(results.iter().map(|result| {
                    let card_content = row![
                        container(text("📖").size(18))
                            .width(Length::Fixed(30.0))
                            .height(Length::Fixed(30.0))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                                border: iced::Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                        bold_text(result).size(15).color(theme::TEXT_SLATE_50),
                    ]
                    .spacing(15)
                    .align_y(Alignment::Center);

                    button(card_content)
                        .on_press(Message::LoadBookDetails(result.clone()))
                        .padding(12)
                        .width(Length::Fill)
                        .style(|_, status| button::Style {
                            background: Some(iced::Background::Color(
                                if status == button::Status::Hovered {
                                    theme::BG_SLATE_700
                                } else {
                                    theme::BG_SLATE_800
                                },
                            )),
                            border: iced::Border {
                                color: theme::BG_SLATE_700,
                                width: 1.0,
                                radius: 8.0.into(),
                            },
                            ..Default::default()
                        })
                        .into()
                }))
                .spacing(8);

                scrollable(results_col).into()
            }
        } else {
            container(
                text("Enter a search term above.")
                    .size(16)
                    .color(theme::TEXT_SLATE_400),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
        };

        column![
            bold_text("Search Books")
                .size(26)
                .color(theme::TEXT_SLATE_50),
            space_y(15.0),
            search_bar,
            space_y(20.0),
            results_content
        ]
        .spacing(10)
        .into()
    }

    fn render_settings(&self) -> Element<'_, Message> {
        let url_input = text_input("https://github.com/owner/repo", &self.github_url)
            .on_input(Message::GithubUrlChanged)
            .on_submit(Message::ImportSources)
            .padding(12)
            .style(|_, status| text_input::Style {
                background: iced::Background::Color(theme::BG_SLATE_850),
                border: iced::Border {
                    color: if matches!(status, text_input::Status::Focused { .. }) {
                        theme::ACCENT_INDIGO_500
                    } else {
                        theme::BG_SLATE_700
                    },
                    width: 1.0,
                    radius: 6.0.into(),
                },
                value: theme::TEXT_SLATE_50,
                placeholder: theme::TEXT_SLATE_400,
                selection: theme::ACCENT_INDIGO_500,
                icon: theme::TEXT_SLATE_400,
            });

        let import_button = if self.is_importing {
            button(text("Importing...").color(theme::TEXT_SLATE_400))
                .padding(12)
                .style(|_, _| button::Style {
                    background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
        } else {
            let is_disabled = self.github_url.trim().is_empty();
            let mut btn = button(text("📥 Import Sources").color(theme::TEXT_SLATE_50)).padding(12);

            if !is_disabled {
                btn = btn
                    .on_press(Message::ImportSources)
                    .style(|_, status| button::Style {
                        background: Some(iced::Background::Color(
                            if status == button::Status::Hovered {
                                theme::ACCENT_INDIGO_600
                            } else {
                                theme::ACCENT_INDIGO_500
                            },
                        )),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
            } else {
                btn = btn.style(|_, _| button::Style {
                    background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                    border: iced::Border {
                        color: theme::BG_SLATE_700,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                });
            }
            btn
        };

        let status_box: Element<'_, Message> = match &self.import_status {
            Some(Ok(success_msg)) => {
                container(text(success_msg).size(13).color(theme::TEXT_SLATE_50))
                    .padding(12)
                    .width(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(theme::SUCCESS_GREEN)),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            }
            Some(Err(err_msg)) => container(text(err_msg).size(13).color(theme::TEXT_SLATE_50))
                .padding(12)
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(theme::ERROR_RED)),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
            None => space_y(0.0).into(),
        };

        let importer_card = container(
            column![
                bold_text("GitHub Source Importer")
                    .size(18)
                    .color(theme::TEXT_SLATE_50),
                space_y(6.0),
                text("Pasting a GitHub repository URL will scan its `sources/` folder, download extension scripts and metadata, and install them locally.")
                    .size(13)
                    .color(theme::TEXT_SLATE_400),
                space_y(15.0),
                row![url_input, import_button].spacing(10).width(Length::Fill),
                space_y(10.0),
                status_box,
            ]
            .spacing(5)
        )
        .padding(20)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(theme::BG_SLATE_800)),
            border: iced::Border {
                color: theme::BG_SLATE_700,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        let sources_list = column(self.sources.iter().map(|s| {
            let mut info = column![
                bold_text(&s.source.name)
                    .size(15)
                    .color(theme::TEXT_SLATE_50)
            ]
            .spacing(2);
            if let Some(desc) = &s.source.description {
                info = info.push(text(desc).size(12).color(theme::TEXT_SLATE_400));
            }

            container(
                row![
                    info,
                    space_fill_x(),
                    text(&s.source.url).size(11).color(theme::TEXT_SLATE_400),
                ]
                .align_y(Alignment::Center),
            )
            .padding(12)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(theme::BG_SLATE_850)),
                border: iced::Border {
                    color: theme::BG_SLATE_700,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into()
        }))
        .spacing(8);

        let active_sources_card = container(
            column![
                bold_text("Active Sources")
                    .size(18)
                    .color(theme::TEXT_SLATE_50),
                space_y(6.0),
                text("These are the currently loaded content sources for finding and reading web novels.")
                    .size(13)
                    .color(theme::TEXT_SLATE_400),
                space_y(15.0),
                scrollable(sources_list).height(Length::FillPortion(1)),
            ]
            .spacing(5)
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(theme::BG_SLATE_800)),
            border: iced::Border {
                color: theme::BG_SLATE_700,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        column![
            bold_text("Sources & Settings")
                .size(26)
                .color(theme::TEXT_SLATE_50),
            space_y(20.0),
            importer_card,
            space_y(20.0),
            active_sources_card,
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn render_reader<'a>(
        &'a self,
        chapter: &'a book_core::models::Chapter,
        blocks: &'a [crate::html_parser::ReaderBlock],
    ) -> Element<'a, Message> {
        let back_btn = button(bold_text("← Back to Book").size(14).color(theme::TEXT_SLATE_50))
            .on_press(Message::CloseReader)
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(
                    if status == button::Status::Hovered {
                        theme::ACCENT_INDIGO_600
                    } else {
                        theme::ACCENT_INDIGO_500
                    },
                )),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(10);

        let (source_id, book_id) = if let Some(book) = &self.viewed_book {
            (book.source_id().to_string(), book.id().to_string())
        } else {
            (String::new(), String::new())
        };

        // Navigation buttons
        let mut prev_btn = button(bold_text("← Previous Chapter").size(14).color(theme::TEXT_SLATE_400))
            .padding(10)
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let mut next_btn = button(bold_text("Next Chapter →").size(14).color(theme::TEXT_SLATE_400))
            .padding(10)
            .style(|_, _| button::Style {
                background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        if let Some(Book::WebNovel(ref webnovel)) = self.viewed_book {
            if let Some(pos) = webnovel.chapters.iter().position(|c| c.id == chapter.id) {
                if pos > 0 {
                    let prev_chap = &webnovel.chapters[pos - 1];
                    prev_btn = button(bold_text("← Previous Chapter").size(14).color(theme::TEXT_SLATE_50))
                        .on_press(Message::LoadChapter(prev_chap.id.clone(), prev_chap.title.clone(), false))
                        .style(|_, status| button::Style {
                            background: Some(iced::Background::Color(
                                if status == button::Status::Hovered {
                                    theme::ACCENT_INDIGO_600
                                } else {
                                    theme::ACCENT_INDIGO_500
                                }
                            )),
                            border: iced::Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .padding(10);
                }
                if pos + 1 < webnovel.chapters.len() {
                    let next_chap = &webnovel.chapters[pos + 1];
                    next_btn = button(bold_text("Next Chapter →").size(14).color(theme::TEXT_SLATE_50))
                        .on_press(Message::LoadChapter(next_chap.id.clone(), next_chap.title.clone(), false))
                        .style(|_, status| button::Style {
                            background: Some(iced::Background::Color(
                                if status == button::Status::Hovered {
                                    theme::ACCENT_INDIGO_600
                                } else {
                                    theme::ACCENT_INDIGO_500
                                }
                            )),
                            border: iced::Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .padding(10);
                }
            }
        }

        let nav_row = row![prev_btn, space_fill_x(), next_btn]
            .spacing(20)
            .width(Length::Fill);

        let mut reader_column = column![].spacing(15).width(Length::Fill);

        for block in blocks {
            match block {
                crate::html_parser::ReaderBlock::Paragraph(spans) => {
                    let mut spans_vec: Vec<iced::widget::text::Span<'_, ()>> = Vec::new();
                    for span_item in spans {
                        match span_item {
                            crate::html_parser::ReaderSpan::Text(t) => {
                                spans_vec.push(span(t.clone()).color(theme::TEXT_SLATE_300).size(16));
                            }
                            crate::html_parser::ReaderSpan::Bold(t) => {
                                spans_vec.push(span(t.clone()).color(theme::TEXT_SLATE_50).size(16).font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                }));
                            }
                            crate::html_parser::ReaderSpan::Italic(t) => {
                                spans_vec.push(span(t.clone()).color(theme::TEXT_SLATE_300).size(16).font(iced::Font {
                                    style: iced::font::Style::Italic,
                                    ..Default::default()
                                }));
                            }
                            crate::html_parser::ReaderSpan::BoldItalic(t) => {
                                spans_vec.push(span(t.clone()).color(theme::TEXT_SLATE_50).size(16).font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    style: iced::font::Style::Italic,
                                    ..Default::default()
                                }));
                            }
                        }
                    }
                    reader_column = reader_column.push(rich_text(spans_vec));
                }
                crate::html_parser::ReaderBlock::Heading(t, level) => {
                    let size = match level {
                        1 => 22,
                        2 => 20,
                        3 => 18,
                        _ => 16,
                    };
                    reader_column = reader_column.push(
                        text(t.clone())
                            .size(size)
                            .color(theme::TEXT_SLATE_50)
                            .font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            })
                    );
                }
                crate::html_parser::ReaderBlock::Image(url) => {
                    let local_path = book_core::storage::webnovel_image_path(&source_id, &book_id, url);
                    if local_path.exists() {
                        reader_column = reader_column.push(
                            Image::new(iced::widget::image::Handle::from_path(local_path))
                                .content_fit(iced::ContentFit::Contain)
                                .width(Length::Fill)
                        );
                    } else {
                        reader_column = reader_column.push(
                            container(
                                text("📷 Loading Image...").size(14).color(theme::TEXT_SLATE_400)
                            )
                            .padding(10)
                            .style(|_| container::Style {
                                background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                                ..Default::default()
                            })
                        );
                    }
                }
            }
        }

        let content_col = column![
            row![back_btn, space_fill_x()].align_y(Alignment::Center),
            space_y(20.0),
            bold_text(&chapter.title).size(24).color(theme::TEXT_SLATE_50),
            space_y(15.0),
            scrollable(
                container(reader_column)
                    .width(Length::Fill)
                    .padding(10)
            )
            .id(iced::widget::Id::new("reader_scroll"))
            .on_scroll(Message::ReaderScrolled)
            .height(Length::Fill),
            space_y(20.0),
            nav_row
        ]
        .spacing(10)
        .padding(10);

        container(content_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(theme::BG_SLATE_900)),
                ..Default::default()
            })
            .into()
    }
}

