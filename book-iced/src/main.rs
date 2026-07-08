pub mod theme;
pub mod helpers;
pub mod book_render;
pub mod app;

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
