use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Task};
use std::sync::Arc;

use book_core::database::Database;
use book_core::models::{Book, Novel, Source, WebNovel};
use book_core::{HomeSection, SourceWithConfig, defaults, importer};
mod theme {
    use iced::Color;

    pub const BG_SLATE_900: Color = Color::from_rgb(15.0 / 255.0, 23.0 / 255.0, 42.0 / 255.0);
    pub const BG_SLATE_800: Color = Color::from_rgb(30.0 / 255.0, 41.0 / 255.0, 59.0 / 255.0);
    pub const BG_SLATE_850: Color = Color::from_rgb(22.0 / 255.0, 32.0 / 255.0, 51.0 / 255.0);
    pub const BG_SLATE_700: Color = Color::from_rgb(51.0 / 255.0, 65.0 / 255.0, 85.0 / 255.0);

    pub const ACCENT_INDIGO_500: Color =
        Color::from_rgb(99.0 / 255.0, 102.0 / 255.0, 241.0 / 255.0);
    pub const ACCENT_INDIGO_600: Color = Color::from_rgb(79.0 / 255.0, 70.0 / 255.0, 229.0 / 255.0);

    pub const TEXT_SLATE_50: Color = Color::from_rgb(248.0 / 255.0, 250.0 / 255.0, 252.0 / 255.0);
    pub const TEXT_SLATE_300: Color = Color::from_rgb(203.0 / 255.0, 213.0 / 255.0, 225.0 / 255.0);
    pub const TEXT_SLATE_400: Color = Color::from_rgb(148.0 / 255.0, 163.0 / 255.0, 184.0 / 255.0);

    pub const SUCCESS_GREEN: Color = Color::from_rgb(16.0 / 255.0, 185.0 / 255.0, 129.0 / 255.0);
    pub const ERROR_RED: Color = Color::from_rgb(239.0 / 255.0, 68.0 / 255.0, 68.0 / 255.0);
}

fn bold_text<'a>(label: &'a str) -> iced::widget::Text<'a> {
    text(label).font(iced::Font {
        weight: iced::font::Weight::Bold,
        ..Default::default()
    })
}

fn space_y(y: f32) -> Space {
    Space::new().height(y)
}
fn _space_x(x: f32) -> Space {
    Space::new().width(x)
}
fn space_fill_x() -> Space {
    Space::new().width(Length::Fill)
}
fn space_fill_y() -> Space {
    Space::new().height(Length::Fill)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Library,
    Discover,
    Search,
    Settings,
}

#[derive(Clone)]
enum Message {
    DatabaseInitialized(Arc<Database>, Vec<SourceWithConfig>, Vec<Book>),
    TabSelected(Tab),
    LoadLibrary,
    LibraryLoaded(Vec<Book>),
    LoadDiscoverData,
    DiscoverDataLoaded(Option<Vec<HomeSection>>),
    GithubUrlChanged(String),
    ImportSources,
    SourcesImported(Result<Vec<String>, String>),
    SourcesLoaded(Vec<SourceWithConfig>),
    SourceSelected(String),
    SearchKeywordChanged(String),
    TriggerSearch,
    SearchResultsLoaded(Option<Vec<book_core::SearchResult>>),
    BookFetched(Option<Book>),
}

struct MyApp {
    active_tab: Tab,
    sources: Vec<SourceWithConfig>,
    discover_sections: Vec<HomeSection>,
    books: Vec<Book>,
    is_loading_library: bool,
    is_loading_discover: bool,
    database: Option<Arc<Database>>, // Changed to Option to handle async init
    github_url: String,
    import_status: Option<Result<String, String>>,
    is_importing: bool,
    selected_source_id: Option<String>,
    search_keyword: String,
    search_results: Option<Vec<book_core::SearchResult>>,
    is_searching: bool,
}

impl MyApp {
    // In iced, `new` cannot be async. We return a Task that initializes the DB.
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
        };

        let task = Task::perform(
            async {
                let db = Database::open_local()
                    .await
                    .expect("Failed to open local database");
                let sources = db.get_sources().await.unwrap_or_default();
                //sources.push(book_core::defaults::novelfire_source());
                let library_books = db.get_library_books().await.unwrap_or_default();
                (Arc::new(db), sources, library_books)
            },
            |(db, sources, books)| Message::DatabaseInitialized(db, sources, books),
        );

        (app, task)
    }

    // `update` is no longer async. We use `Task::perform` to run async operations.
    fn update(&mut self, message: Message) -> Task<Message> {
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
                match tab {
                    Tab::Library if self.books.is_empty() => Task::done(Message::LoadLibrary),
                    Tab::Discover if self.discover_sections.is_empty() => {
                        Task::done(Message::LoadDiscoverData)
                    }
                    _ => Task::none(),
                }
            }
            Message::LoadLibrary => {
                // Safely grab the database clone only when needed
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
                self.books.extend(books);
                Task::none()
            }
            Message::LoadDiscoverData => {
                self.is_loading_discover = true;
                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                if let Some(source) = source
                    && (source.source.id != "local")
                {
                    println!("still running");
                    Task::perform(
                        async move { book_core::api::get_discover_page(&source).await },
                        Message::DiscoverDataLoaded,
                    )
                } else {
                    self.is_loading_discover = false;
                    Task::none()
                }
            }
            Message::DiscoverDataLoaded(sections) => {
                let source = self
                    .sources
                    .iter()
                    .find(|s| Some(&s.source.id) == self.selected_source_id.as_ref())
                    .or_else(|| self.sources.first())
                    .cloned();

                self.is_loading_discover = false;
                self.discover_sections = sections.unwrap_or_default();

                let ids: Vec<String> = self
                    .discover_sections
                    .iter()
                    .flat_map(|section| section.books.clone())
                    .collect();

                // 1. SAFELY get the database to prevent panics
                let db = match &self.database {
                    Some(db) => Arc::clone(db),
                    None => return Task::none(), // DB not ready, abort gracefully
                };

                // 2. PERFORMANCE: Use a HashSet for O(1) lookups instead of scanning the Vec
                let existing_ids: std::collections::HashSet<_> =
                    self.books.iter().map(|b| b.id().to_string()).collect();

                // 3. CONCURRENCY: Filter missing IDs and limit to 10 at a time
                // to prevent API rate limits and network storms
                let ids_to_fetch: Vec<String> = ids
                    .into_iter()
                    .filter(|id| !existing_ids.contains(id))
                    .take(10)
                    .collect();

                let fetch_tasks: Vec<Task<Message>> = ids_to_fetch
                    .into_iter()
                    .filter_map(|id| {
                        source.clone().map(|src| {
                            let db = Arc::clone(&db);
                            Task::perform(
                                async move { book_core::api::get_book(&*db, &src, &id).await },
                                Message::BookFetched,
                            )
                        })
                    })
                    .collect();

                Task::batch(fetch_tasks)
            }
            // Handle the result when the background tasks finish
            Message::BookFetched(result) => {
                match result {
                    Some(book) => {
                        println!("{:?}", &book.id());
                        self.books.push(book);
                    }
                    None => {
                        println!("Failed to fetch book: ");
                        // Optionally: set an error flag in your model to show a UI toast/alert
                    }
                }
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

                // Safely clone here where it's needed
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
                    let keyword = self.search_keyword.clone();
                    Task::perform(
                        async move { book_core::api::search_books(&source, &keyword, None).await },
                        Message::SearchResultsLoaded,
                    )
                } else {
                    Task::none()
                }
            }
            Message::SearchResultsLoaded(results) => {
                self.is_searching = false;
                self.search_results = results;
                Task::none()
            }
        }
    }
    // --- View Methods (Unchanged) ---
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

    fn view(&self) -> Element<'_, Message> {
        let sidebar = container(
            column![
                // App Logo / Title
                container(bold_text("novel app").size(20).color(theme::TEXT_SLATE_50))
                    .padding(20)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                // Navigation Options
                self.nav_button("Library", Tab::Library, self.active_tab == Tab::Library),
                self.nav_button("Discover", Tab::Discover, self.active_tab == Tab::Discover),
                self.nav_button("Search", Tab::Search, self.active_tab == Tab::Search),
                self.nav_button(
                    "Sources & Settings",
                    Tab::Settings,
                    self.active_tab == Tab::Settings
                ),
                space_fill_y(),
                // Footer
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

        let main_content = container(match self.active_tab {
            Tab::Library => self.render_library(),
            Tab::Discover => self.render_discover(),
            Tab::Search => self.render_search(),
            Tab::Settings => self.render_settings(),
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

        let content: Element<'_, Message> = if self.is_loading_library {
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
            let books_col = column(self.books.iter().map(|book| {
                let base = book.base();
                let cover = container(text("📖").size(32))
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
                    });

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

                container(
                    row![cover, book_info]
                        .spacing(15)
                        .align_y(Alignment::Center),
                )
                .padding(12)
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                    border: iced::Border {
                        color: theme::BG_SLATE_700,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
                .into()
            }))
            .spacing(10);

            scrollable(books_col).into()
        };

        column![title_row, space_y(20.0), content]
            .spacing(10)
            .into()
    }

    fn render_discover(&self) -> Element<'_, Message> {
        let source_selector: Element<'_, Message> = if self.sources.is_empty() {
            text("No sources configured. Please import or add a source in settings.")
                .color(theme::ERROR_RED)
                .into()
        } else {
            let source_buttons = row(self.sources.iter().map(|s| {
                let is_selected = self
                    .selected_source_id
                    .as_ref()
                    .map(|id| id == &s.source.id)
                    .unwrap_or_else(|| {
                        self.sources.first().map(|fs| &fs.source.id) == Some(&s.source.id)
                    });

                button(text(&s.source.name).size(13).color(theme::TEXT_SLATE_50))
                    .on_press(Message::SourceSelected(s.source.id.clone()))
                    .padding(8)
                    .style(move |_, status| button::Style {
                        background: Some(iced::Background::Color(if is_selected {
                            theme::ACCENT_INDIGO_500
                        } else if status == button::Status::Hovered {
                            theme::BG_SLATE_700
                        } else {
                            theme::BG_SLATE_800
                        })),
                        border: iced::Border {
                            color: theme::BG_SLATE_700,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    })
                    .into()
            }))
            .spacing(10);

            let horiz_direction =
                scrollable::Direction::Horizontal(iced::widget::scrollable::Scrollbar::default());
            scrollable(source_buttons).direction(horiz_direction).into()
        };

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
                    // println!("{}", book_name);
                    // println!("{:?}", self.books.first());
                    match book {
                        Some(Book::Novel(novel)) => novel.render_card(),
                        Some(Book::WebNovel(wn)) => wn.render_card(),
                        None => text(book_name).into(),
                    }
                    // container(
                    //     column![
                    //         container(text("📖").size(24))
                    //             .width(Length::Fixed(40.0))
                    //             .height(Length::Fixed(40.0))
                    //             .align_x(Alignment::Center)
                    //             .align_y(Alignment::Center)
                    //             .style(|_| container::Style {
                    //                 background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                    //                 border: iced::Border {
                    //                     radius: 4.0.into(),
                    //                     ..Default::default()
                    //                 },
                    //                 ..Default::default()
                    //             }),
                    //         space_y(5.0),
                    //         text(book_name)
                    //             .size(12)
                    //             .font(iced::Font {
                    //                 weight: iced::font::Weight::Bold,
                    //                 ..Default::default()
                    //             })
                    //             .color(theme::TEXT_SLATE_300)
                    //             .width(Length::Fixed(120.0)),
                    //     ]
                    //     .align_x(Alignment::Center),
                    // )
                    // .padding(10)
                    // .width(Length::Fixed(140.0))
                    // .style(|_| container::Style {
                    //     background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                    //     border: iced::Border {
                    //         color: theme::BG_SLATE_700,
                    //         width: 1.0,
                    //         radius: 6.0.into(),
                    //     },
                    //     ..Default::default()
                    // })
                    .into()
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
                    container(
                        row![
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
                        .align_y(Alignment::Center),
                    )
                    .padding(12)
                    .width(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(theme::BG_SLATE_800)),
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
}

fn get_theme(_app: &MyApp) -> iced::Theme {
    iced::Theme::Dark
}

pub fn main() -> iced::Result {
    iced::application(MyApp::new, MyApp::update, MyApp::view)
        .title("Antigravity Novel Reader")
        .theme(get_theme)
        .run()
}
pub trait RenderIcedBook<Message: 'static> {
    /// Renders the external view (Card) for library grids/lists
    fn render_card(&self) -> Element<'_, Message>;

    /// Renders the detailed view (Page) when the book is clicked
    fn render_detail(&self) -> Element<'_, Message>;
}

impl<Message: 'static> RenderIcedBook<Message> for Novel {
    fn render_card(&self) -> Element<'_, Message> {
        column![
            // Note: iced's `image` widget typically requires a local path or bytes.
            // If `cover_url` is a URL, you might need a custom widget or async image loader.
            // image(&self.base.cover_url).width(120).height(160),
            text(&self.base.title).size(20),
            text(&self.base.author).size(14),
            text(format!("Rating: {:.1} ⭐", self.base.rating)).size(12),
            text(format!("Progress: {:.0}%", self.progress * 100.0)).size(12),
        ]
        .spacing(5)
        .padding(10)
        .into()
    }

    fn render_detail(&self) -> Element<'_, Message> {
        column![
            text(&self.base.title).size(28),
            text(&self.base.author).size(18),
            text(format!("Format: {:?}", self.format)),
            text(format!(
                "File Path: {}",
                self.file_path.as_deref().unwrap_or("Not downloaded yet")
            )),
            text(format!("Progress: {:.0}%", self.progress * 100.0)),
            text("Summary:").size(18),
            text(&self.base.summary),
            text(format!("Genres: {}", self.base.genres.join(", "))),
            text(format!("Status: {}", self.base.status)),
        ]
        .spacing(10)
        .padding(20)
        .into()
    }
}

impl<Message: 'static> RenderIcedBook<Message> for WebNovel {
    fn render_card(&self) -> Element<'_, Message> {
        column![
            // image(&self.base.cover_url).width(120).height(160),
            text(&self.base.title).size(20),
            text(&self.base.author).size(14),
            text(format!("Rating: {:.1} ⭐", self.base.rating)).size(12),
            text(format!("Chapters: {}", self.chapters_count)).size(12),
        ]
        .spacing(5)
        .padding(10)
        .into()
    }

    fn render_detail(&self) -> Element<'_, Message> {
        let mut details = column![
            text(&self.base.title).size(28),
            text(&self.base.author).size(18),
            text(format!("Chapters Count: {}", self.chapters_count)),
            text("Summary:").size(18),
            text(&self.base.summary),
            text(format!("Genres: {}", self.base.genres.join(", "))),
            text(format!("Status: {}", self.base.status)),
            text("Chapters List:").size(18),
        ]
        .spacing(10);

        // Append chapters to the column
        for chapter in &self.chapters {
            details = details.push(
                // You can wrap this in a `button` later to trigger a "ReadChapter" message
                text(format!("- {}", chapter.title)),
            );
        }

        scrollable(details.padding(20)).height(Length::Fill).into()
    }
}
