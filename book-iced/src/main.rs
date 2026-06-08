use iced::widget::{button, center, column, container, row, text};
use iced::{Alignment, Center, Element, Length, Task};
use iced::window;

// Simulating your book_core types for context
use book_core::{SourceWithConfig, api::*};
use book_core::database::Database;
use book_core::models::{Book, Chapter, ChapterContent};

pub fn main() -> iced::Result {
    // Pass the initialization logic directly as the first argument
    iced::application(MyApp::new, MyApp::update, MyApp::view)
        .title("Book App")
        .run()
}

// 1. Update your application state struct to hold the initialized data
struct MyApp {
    database: Database,
    sources: Vec<SourceWithConfig>,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Confirm,
    Exit,
}

impl MyApp {
    // 2. Properly handle data initialization inside the boot function
    fn new() -> (Self, Task<Message>) {
        let database = Database::new().unwrap();
        let mut sources = database.get_sources().unwrap();
        if sources.len() == 0 {sources.push(book_core::defaults::novelfire_source());}
        (
            Self { database, sources },
            Task::none(), // No async tasks needed immediately on startup
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Confirm => window::latest().and_then(window::close),
            Message::Exit => Task::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        // Map every source into its UI element (now correctly references self.sources)
        let list_content = column(
            self.sources
                .iter()
                .map(|source| source.view())
                .collect::<Vec<_>>()
        )
        .spacing(10); // Space between each source row

        // Wrap the list in a container
        container(list_content)
            .padding(20)
            .into()
    }
}

// 3. Define the trait for external rendering compatibility
pub trait RenderIce {
    fn view(&self) -> Element<Message>;
}

// 4. Implement that trait for the external struct
impl RenderIce for SourceWithConfig {
    fn view(&self) -> Element<Message> {
        let mut info = column![
            text(&self.source.name).size(18),
        ].spacing(4);

        if let Some(desc) = &self.source.description {
            info = info.push(text(desc).size(14));
        }

        let mut row_content = row![].spacing(12);
        row_content = row_content
            .push(info)
            .push(text(&self.source.url).size(12));

        container(row_content)
            .padding(10)
            .into()
    }
}

