pub mod app;
pub mod book_render;
pub mod helpers;
pub mod html_parser;
pub mod theme;

use app::MyApp;

fn get_theme(_app: &MyApp) -> iced::Theme {
    iced::Theme::Dark
}

pub fn main() -> iced::Result {
    iced::application(MyApp::new, MyApp::update, MyApp::view)
        .title("Antigravity Novel Reader")
        .theme(get_theme)
        .run()
}
