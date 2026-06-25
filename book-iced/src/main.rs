use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Task};
use std::thread;
use tokio::sync::{mpsc, oneshot};

use book_core::database::Database;
use book_core::models::Book;
use book_core::{HomeSection, SourceWithConfig};

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

#[derive(Debug, Clone)]
enum Message {
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
}

pub enum DbCommand {
    GetLibraryBooks {
        responder: oneshot::Sender<Vec<Book>>,
    },
    ImportFromGithub {
        repo_url: String,
        base_dir: std::path::PathBuf,
        responder: oneshot::Sender<Result<Vec<String>, String>>,
    },
    GetSources {
        responder: oneshot::Sender<Vec<SourceWithConfig>>,
    },
}

pub enum NetCommand {
    FetchDiscoverPage {
        source: SourceWithConfig,
        responder: oneshot::Sender<Option<Vec<HomeSection>>>,
    },
    SearchBooks {
        source: SourceWithConfig,
        keyword: String,
        responder: oneshot::Sender<Option<Vec<book_core::SearchResult>>>,
    },
}

struct MyApp {
    db_tx: mpsc::Sender<DbCommand>,
    net_tx: mpsc::Sender<NetCommand>,
    active_tab: Tab,
    sources: Vec<SourceWithConfig>,
    discover_sections: Vec<HomeSection>,
    library_books: Vec<Book>,
    is_loading_library: bool,
    is_loading_discover: bool,

    // Importer & Search State
    github_url: String,
    import_status: Option<Result<String, String>>,
    is_importing: bool,
    selected_source_id: Option<String>,
    search_keyword: String,
    search_results: Option<Vec<book_core::SearchResult>>,
    is_searching: bool,
}

impl MyApp {
    pub fn new() -> (Self, Task<Message>) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create startup runtime");

        let (mut sources, library_books) = rt.block_on(async {
            let database = Database::open_local()
                .await
                .expect("Failed to open local database");
            let sources = database.get_sources().await.unwrap_or_default();
            let library_books = database.get_library_books().await.unwrap_or_default();
            (sources, library_books)
        });

        if sources.is_empty() {
            sources.push(book_core::defaults::novelfire_source());
        }

        let selected_source_id = sources.first().map(|s| s.source.id.clone());

        // DB thread
        let (db_tx, mut db_rx) = mpsc::channel::<DbCommand>(100);
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create database runtime");

            rt.block_on(async move {
                let db = Database::open_local()
                    .await
                    .expect("Failed to open local database");

                while let Some(command) = db_rx.recv().await {
                    match command {
                        DbCommand::GetLibraryBooks { responder } => {
                            let books = book_core::api::get_library_books(&db)
                                .await
                                .unwrap_or_default();
                            let _ = responder.send(books);
                        }
                        DbCommand::ImportFromGithub {
                            repo_url,
                            base_dir,
                            responder,
                        } => {
                            let result = book_core::import_from_github(&repo_url, &base_dir, &db)
                                .await
                                .map_err(|e| e.to_string());
                            let _ = responder.send(result);
                        }
                        DbCommand::GetSources { responder } => {
                            let sources = db.get_sources().await.unwrap_or_default();
                            let _ = responder.send(sources);
                        }
                    }
                }
            });
        });

        // Network thread
        let (net_tx, mut net_rx) = mpsc::channel::<NetCommand>(32);
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create network runtime");

            rt.block_on(async move {
                while let Some(cmd) = net_rx.recv().await {
                    match cmd {
                        NetCommand::FetchDiscoverPage { source, responder } => {
                            let result = book_core::api::get_discover_page(&source).await;
                            println!("loading home called from GUI");
                            let _ = responder.send(result);
                        }
                        NetCommand::SearchBooks {
                            source,
                            keyword,
                            responder,
                        } => {
                            let result =
                                book_core::api::search_books(&source, &keyword, None).await;
                            let _ = responder.send(result);
                        }
                    }
                }
            });
        });

        let app = Self {
            db_tx,
            net_tx,
            sources,
            active_tab: Tab::Library,
            library_books,
            is_loading_library: false,
            is_loading_discover: false,
            discover_sections: Vec::new(),
            github_url: String::new(),
            import_status: None,
            is_importing: false,
            selected_source_id,
            search_keyword: String::new(),
            search_results: None,
            is_searching: false,
        };

        let initial_task = Task::perform(async {}, |_| Message::LoadLibrary);
        (app, initial_task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                match tab {
                    Tab::Library => {
                        if self.library_books.is_empty() {
                            return Task::perform(async {}, |_| Message::LoadLibrary);
                        }
                    }
                    Tab::Discover => {
                        if self.discover_sections.is_empty() {
                            return Task::perform(async {}, |_| Message::LoadDiscoverData);
                        }
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::LoadLibrary => {
                self.is_loading_library = true;
                let tx = self.db_tx.clone();

                Task::future(async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx
                        .send(DbCommand::GetLibraryBooks { responder: resp_tx })
                        .await;
                    let books = resp_rx.await.unwrap_or_default();
                    Message::LibraryLoaded(books)
                })
            }

            Message::LibraryLoaded(books) => {
                self.is_loading_library = false;
                self.library_books = books;
                Task::none()
            }

            Message::LoadDiscoverData => {
                self.is_loading_discover = true;
                let tx = self.net_tx.clone();
                let source = match &self.selected_source_id {
                    Some(id) => self.sources.iter().find(|s| s.source.id == *id).cloned(),
                    None => self.sources.first().cloned(),
                };
                let source = match source {
                    Some(s) => s,
                    None => {
                        self.is_loading_discover = false;
                        return Task::none();
                    }
                };
                Task::future(async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx
                        .send(NetCommand::FetchDiscoverPage {
                            source,
                            responder: resp_tx,
                        })
                        .await;
                    let result = resp_rx.await.unwrap_or(None);
                    Message::DiscoverDataLoaded(result)
                })
            }

            Message::DiscoverDataLoaded(sections) => {
                self.is_loading_discover = false;
                if let Some(data) = sections {
                    self.discover_sections = data;
                } else {
                    self.discover_sections.clear();
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
                self.is_importing = true;
                self.import_status = None;
                let tx = self.db_tx.clone();
                let repo_url = self.github_url.clone();
                let base_dir = std::env::current_dir().unwrap_or_default();

                Task::future(async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx
                        .send(DbCommand::ImportFromGithub {
                            repo_url,
                            base_dir,
                            responder: resp_tx,
                        })
                        .await;
                    let result = resp_rx
                        .await
                        .unwrap_or(Err("Database channel closed".to_string()));
                    Message::SourcesImported(result)
                })
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

                        // Trigger reload of sources
                        let tx = self.db_tx.clone();
                        return Task::future(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();
                            let _ = tx.send(DbCommand::GetSources { responder: resp_tx }).await;
                            let sources = resp_rx.await.unwrap_or_default();
                            Message::SourcesLoaded(sources)
                        });
                    }
                    Err(err) => {
                        self.import_status = Some(Err(format!("Import failed: {}", err)));
                    }
                }
                Task::none()
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
                Task::perform(async {}, |_| Message::LoadDiscoverData)
            }

            Message::SearchKeywordChanged(keyword) => {
                self.search_keyword = keyword;
                Task::none()
            }

            Message::TriggerSearch => {
                if self.search_keyword.trim().is_empty() {
                    return Task::none();
                }
                let source = match &self.selected_source_id {
                    Some(id) => self.sources.iter().find(|s| s.source.id == *id).cloned(),
                    None => self.sources.first().cloned(),
                };
                let source = match source {
                    Some(s) => s,
                    None => return Task::none(),
                };
                self.is_searching = true;
                self.search_results = None;
                let tx = self.net_tx.clone();
                let keyword = self.search_keyword.clone();
                Task::future(async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx
                        .send(NetCommand::SearchBooks {
                            source,
                            keyword,
                            responder: resp_tx,
                        })
                        .await;
                    let results = resp_rx.await.unwrap_or(None);
                    Message::SearchResultsLoaded(results)
                })
            }

            Message::SearchResultsLoaded(results) => {
                self.is_searching = false;
                self.search_results = results;
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

    fn view(&self) -> Element<'_, Message> {
        let sidebar = container(
            column![
                // App Logo / Title
                container(
                    bold_text("📚 ANTIGRAVITY")
                        .size(20)
                        .color(theme::TEXT_SLATE_50)
                )
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
        } else if self.library_books.is_empty() {
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
            let books_col = column(self.library_books.iter().map(|book| {
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
                    container(
                        column![
                            container(text("📖").size(24))
                                .width(Length::Fixed(40.0))
                                .height(Length::Fixed(40.0))
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
                            space_y(5.0),
                            text(book_name)
                                .size(12)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
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
