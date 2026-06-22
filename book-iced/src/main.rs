use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Task};
use std::thread;
use tokio::sync::{mpsc, oneshot};

use book_core::database::Database;
use book_core::models::Book;
use book_core::{HomeSection, SourceWithConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Library,
    Discover,
    Search,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    GeneralSettings,
    Sources,
    None,
}

struct SettingsState {
    active_tab: SettingsTab,
}

#[derive(Debug, Clone)]
enum Message {
    TabSelected(Tab),
    SettingsChange(SettingsTab),
    LoadLibrary,
    LibraryLoaded(Vec<Book>),
    LoadDiscoverData,
    DiscoverDataLoaded(Option<Vec<HomeSection>>),
}

pub enum DbCommand {
    GetLibraryBooks {
        responder: oneshot::Sender<Vec<Book>>,
    },
}

pub enum NetCommand {
    FetchDiscoverPage {
        source: SourceWithConfig,
        responder: oneshot::Sender<Option<Vec<HomeSection>>>,
    },
}

struct MyApp {
    db_tx: mpsc::Sender<DbCommand>,
    net_tx: mpsc::Sender<NetCommand>,
    active_tab: Tab,
    sources: Vec<SourceWithConfig>,
    settings_state: SettingsState,
    discover_sections: Vec<HomeSection>,
    library_books: Vec<Book>,
    is_loading_library: bool,
    is_loading_discover: bool,
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
                    }
                }
            });
        });

        // Network thread — owns its own Tokio runtime so reqwest/hyper work correctly
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
                            println!("{:?}", result);
                            let _ = responder.send(result.ok());
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
            settings_state: SettingsState {
                active_tab: SettingsTab::None,
            },
            library_books,
            is_loading_library: false,
            is_loading_discover: false,
            discover_sections: Vec::new(),
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
                let source = self.sources.first().cloned().unwrap();
                println!("{:?}", source);
                Task::future(async move {
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let _ = tx
                        .send(NetCommand::FetchDiscoverPage {
                            source,
                            responder: resp_tx,
                        })
                        .await;
                    let result = resp_rx.await.unwrap_or(None);
                    println!("{:?}", result);
                    Message::DiscoverDataLoaded(result)
                })
            }

            Message::DiscoverDataLoaded(sections) => {
                self.is_loading_discover = false;
                if let Some(data) = sections {
                    self.discover_sections = data;
                }
                Task::none()
            }

            Message::SettingsChange(tab) => {
                self.settings_state.active_tab = tab;
                Task::none()
            }
        }
    }

    fn nav_button<'a>(&self, label: &'a str, tab: Tab) -> Element<'a, Message> {
        button(text(label).align_x(Alignment::Center))
            .on_press(Message::TabSelected(tab))
            .width(Length::Fill)
            .padding(12)
            .into()
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.active_tab {
            Tab::Library => self.render_home(),
            Tab::Discover => self.render_discover(),
            Tab::Search => self.render_search(),
            Tab::Settings => self.render_settings(),
        };

        let main_area = container(content).width(Length::Fill).height(Length::Fill);

        let nav_bar = row![
            self.nav_button("Home", Tab::Library),
            self.nav_button("Discover", Tab::Discover),
            self.nav_button("Search", Tab::Search),
            self.nav_button("Settings", Tab::Settings),
        ]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        column![main_area, nav_bar].into()
    }

    fn render_home(&self) -> Element<'_, Message> {
        if self.is_loading_library {
            return text("Loading library...").into();
        }
        if self.library_books.is_empty() {
            return text("No books in library.").into();
        }
        scrollable(column(self.library_books.iter().map(|book| book.view()))).into()
    }

    fn render_discover(&self) -> Element<'_, Message> {
        if self.is_loading_discover {
            return text("Loading...").into();
        }
        if self.discover_sections.is_empty() {
            return text("Nothing to show.").into();
        }
        scrollable(column(
            self.discover_sections
                .iter()
                .map(|section| column![text(&section.title).size(18),].into()),
        ))
        .into()
    }

    fn render_search(&self) -> Element<'_, Message> {
        text("search").into()
    }

    fn render_settings(&self) -> Element<'_, Message> {
        match &self.settings_state.active_tab {
            SettingsTab::None => column![
                button("General settings")
                    .on_press(Message::SettingsChange(SettingsTab::GeneralSettings)),
                button("Manage sources").on_press(Message::SettingsChange(SettingsTab::Sources)),
            ]
            .into(),
            SettingsTab::Sources => scrollable(column(
                self.sources
                    .iter()
                    .map(|source| source.view())
                    .collect::<Vec<_>>(),
            ))
            .into(),
            SettingsTab::GeneralSettings => text("General settings").into(),
        }
    }
}

pub fn main() -> iced::Result {
    iced::application(MyApp::new, MyApp::update, MyApp::view)
        .title("Book App")
        .run()
}

trait RenderIce {
    fn view<'a>(&'a self) -> Element<'a, Message>;
}

impl RenderIce for SourceWithConfig {
    fn view<'a>(&'a self) -> Element<'a, Message> {
        let mut info = column![text(&self.source.name).size(18)].spacing(4);
        if let Some(desc) = &self.source.description {
            info = info.push(text(desc).size(14));
        }
        let row_content = row![info, text(&self.source.url).size(12)].spacing(12);
        container(row_content).padding(10).into()
    }
}

impl RenderIce for Book {
    fn view<'a>(&'a self) -> Element<'a, Message> {
        let info = column![
            text(&self.title).size(16),
            text(&self.author).size(14),
            row![
                text(format!("⭐ {:.1}", self.rating)).size(12),
                text(&self.status).size(12),
            ]
            .spacing(10),
        ]
        .spacing(4);

        let cover = container(text("📖 Cover"))
            .width(Length::Fixed(80.0))
            .height(Length::Fixed(110.0));

        row![cover, info].spacing(12).into()
    }
}
