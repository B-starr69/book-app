use iced::widget::{Space, text};
use iced::{Font, Length};

pub fn bold_text<'a>(label: &'a str) -> iced::widget::Text<'a> {
    text(label).font(Font {
        weight: iced::font::Weight::Bold,
        ..Default::default()
    })
}

pub fn space_y(y: f32) -> Space {
    Space::new().height(y)
}
pub fn _space_x(x: f32) -> Space {
    Space::new().width(x)
}
pub fn space_fill_x() -> Space {
    Space::new().width(Length::Fill)
}
pub fn space_fill_y() -> Space {
    Space::new().height(Length::Fill)
}
