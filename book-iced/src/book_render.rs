use iced::{Alignment, Element, Length};
use iced::widget::{column, container, row, scrollable, text, Image, button};
use book_core::models::{Novel, WebNovel};
use crate::theme;
use crate::helpers::{bold_text, space_y, space_fill_x};
use crate::app::Message;

pub trait RenderIcedBook<Message: 'static> {
    /// Renders the external view (Card) for library grids/lists
    fn render_card(&self) -> Element<'_, Message>;

    /// Renders the detailed view (Page) when the book is clicked
    fn render_detail(&self, show_menu: bool) -> Element<'_, Message>;
}

impl RenderIcedBook<Message> for Novel {
    fn render_card(&self) -> Element<'_, Message> {
        let cover_element: Element<'_, Message> = if let Some(path) = book_core::storage::get_cover_path(&self.base.source_id, &self.base.id, &self.base.cover_url) {
            if path.exists() {
                let img: Image = Image::new(iced::widget::image::Handle::from_path(path));
                img.width(120)
                    .height(160)
                    .into()
            } else {
                container(text("📖").size(32))
                    .width(120)
                    .height(160)
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
            }
        } else {
            container(text("📖").size(32))
                .width(120)
                .height(160)
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

        column![
            cover_element,
            text(&self.base.title).size(16).width(120),
            text(&self.base.author).size(12).color(theme::TEXT_SLATE_400).width(120),
            text(format!("Rating: {:.1} ⭐", self.base.rating)).size(10).color(theme::TEXT_SLATE_400),
            text(format!("Progress: {:.0}%", self.progress * 100.0)).size(10).color(theme::TEXT_SLATE_400),
        ]
        .spacing(5)
        .padding(10)
        .into()
    }

    fn render_detail(&self, _show_menu: bool) -> Element<'_, Message> {
        let cover_element: Element<'_, Message> = if let Some(path) = book_core::storage::get_cover_path(&self.base.source_id, &self.base.id, &self.base.cover_url) {
            if path.exists() {
                let img: Image = Image::new(iced::widget::image::Handle::from_path(path));
                img.width(180)
                    .height(240)
                    .into()
            } else {
                container(text("📖").size(48))
                    .width(180)
                    .height(240)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            }
        } else {
            container(text("📖").size(48))
                .width(180)
                .height(240)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
        };

        let library_btn = if self.base.in_library {
            button(bold_text("Remove from Library").size(14).color(theme::TEXT_SLATE_50))
                .on_press(Message::ToggleLibraryStatus(self.base.id.clone(), self.base.source_id.clone(), false))
                .style(|_, status| button::Style {
                    background: Some(iced::Background::Color(
                        if status == button::Status::Hovered {
                            iced::Color::from_rgb8(180, 50, 50)
                        } else {
                            iced::Color::from_rgb8(150, 40, 40)
                        }
                    )),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(8)
        } else {
            button(bold_text("Add to Library").size(14).color(theme::TEXT_SLATE_50))
                .on_press(Message::ToggleLibraryStatus(self.base.id.clone(), self.base.source_id.clone(), true))
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
                .padding(8)
        };

        let metadata_column = column![
            bold_text(&self.base.title).size(28).color(theme::TEXT_SLATE_50),
            text(&self.base.author).size(18).color(theme::TEXT_SLATE_300),
            space_y(10.0),
            text(format!("Format: {:?}", self.format)).color(theme::TEXT_SLATE_400),
            text(format!(
                "File Path: {}",
                self.file_path.as_deref().unwrap_or("Not downloaded yet")
            )).color(theme::TEXT_SLATE_400),
            text(format!("Progress: {:.0}%", self.progress * 100.0)).color(theme::TEXT_SLATE_400),
            text(format!("Rating: {:.1} ⭐", self.base.rating)).color(theme::TEXT_SLATE_300),
            text(format!("Status: {}", self.base.status)).color(theme::TEXT_SLATE_400),
            text(format!("Genres: {}", self.base.genres.join(", "))).color(theme::TEXT_SLATE_400),
            space_y(5.0),
            library_btn,
        ]
        .spacing(8);

        let main_info = row![cover_element, metadata_column]
            .spacing(25)
            .align_y(Alignment::Start);

        scrollable(
            column![
                main_info,
                space_y(20.0),
                bold_text("Summary").size(20).color(theme::TEXT_SLATE_50),
                space_y(5.0),
                text(&self.base.summary).color(theme::TEXT_SLATE_300),
            ]
            .spacing(10)
            .padding(10)
        )
        .into()
    }
}

impl RenderIcedBook<Message> for WebNovel {
    fn render_card(&self) -> Element<'_, Message> {
        let cover_element: Element<'_, Message> = if let Some(path) = book_core::storage::get_cover_path(&self.base.source_id, &self.base.id, &self.base.cover_url) {
            if path.exists() {
                let img: Image = Image::new(iced::widget::image::Handle::from_path(path));
                img.width(120)
                    .height(160)
                    .into()
            } else {
                container(text("📖").size(32))
                    .width(120)
                    .height(160)
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
            }
        } else {
            container(text("📖").size(32))
                .width(120)
                .height(160)
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

        column![
            cover_element,
            text(&self.base.title).size(16).width(120),
            text(&self.base.author).size(12).color(theme::TEXT_SLATE_400).width(120),
            text(format!("Rating: {:.1} ⭐", self.base.rating)).size(10).color(theme::TEXT_SLATE_400),
            text(format!("Chapters: {}", self.chapters_count)).size(10).color(theme::TEXT_SLATE_400),
        ]
        .spacing(5)
        .padding(10)
        .into()
    }

    fn render_detail(&self, show_menu: bool) -> Element<'_, Message> {
        let cover_element: Element<'_, Message> = if let Some(path) = book_core::storage::get_cover_path(&self.base.source_id, &self.base.id, &self.base.cover_url) {
            if path.exists() {
                let img: Image = Image::new(iced::widget::image::Handle::from_path(path));
                img.width(180)
                    .height(240)
                    .into()
            } else {
                container(text("📖").size(48))
                    .width(180)
                    .height(240)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            }
        } else {
            container(text("📖").size(48))
                .width(180)
                .height(240)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(theme::BG_SLATE_700)),
                    border: iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
        };

        let library_btn = if self.base.in_library {
            button(bold_text("Remove from Library").size(14).color(theme::TEXT_SLATE_50))
                .on_press(Message::ToggleLibraryStatus(self.base.id.clone(), self.base.source_id.clone(), false))
                .style(|_theme: &iced::Theme, status| button::Style {
                    background: Some(iced::Background::Color(
                        if status == button::Status::Hovered {
                            iced::Color::from_rgb8(180, 50, 50)
                        } else {
                            iced::Color::from_rgb8(150, 40, 40)
                        }
                    )),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding(8)
        } else {
            button(bold_text("Add to Library").size(14).color(theme::TEXT_SLATE_50))
                .on_press(Message::ToggleLibraryStatus(self.base.id.clone(), self.base.source_id.clone(), true))
                .style(|_theme: &iced::Theme, status| button::Style {
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
                .padding(8)
        };

        // Determine continue reading chapter
        let continue_chapter = if self.chapters.is_empty() {
            None
        } else {
            let last_read_opt = self.chapters.iter()
                .filter(|c| c.last_read > 0)
                .max_by_key(|c| c.last_read);

            match last_read_opt {
                Some(last_chap) => Some(last_chap),
                None => Some(&self.chapters[0]),
            }
        };

        let read_btn = if let Some(chap) = continue_chapter {
            let btn_text = if self.chapters.iter().any(|c| c.last_read > 0) {
                format!("📖 Continue Reading: {}", chap.title)
            } else {
                "📖 Start Reading".to_string()
            };

            button(
                text(btn_text)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .size(14)
                    .color(theme::TEXT_SLATE_50)
            )
            .on_press(Message::LoadChapter(chap.id.clone(), chap.title.clone(), false))
                .style(|_theme: &iced::Theme, status| button::Style {
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
                .padding(8)
        } else {
            button(bold_text("📖 No Chapters").size(14).color(theme::TEXT_SLATE_400))
                .style(|_theme: &iced::Theme, _| button::Style {
                    background: Some(iced::Background::Color(theme::BG_SLATE_800)),
                    border: iced::Border {
                        color: theme::BG_SLATE_700,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                })
                .padding(8)
        };

        let action_row = row![read_btn, library_btn].spacing(10);

        let menu_btn = button(text("⋮").size(24).color(theme::TEXT_SLATE_300))
            .on_press(Message::ToggleBookMenu)
            .style(|_theme: &iced::Theme, status| button::Style {
                background: Some(iced::Background::Color(
                    if status == button::Status::Hovered {
                        theme::BG_SLATE_700
                    } else {
                        iced::Color::TRANSPARENT
                    }
                )),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(5);

        let title_row = row![
            bold_text(&self.base.title).size(28).color(theme::TEXT_SLATE_50),
            space_fill_x(),
            menu_btn,
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill);

        let menu_dropdown: Element<'_, Message> = if show_menu {
            let sync_item: Element<'_, Message> = button(
                row![
                    text("🔄").size(14),
                    text(" Force Sync Content").size(13).color(theme::TEXT_SLATE_50)
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            )
            .on_press(Message::ForceSyncBook(self.base.id.clone()))
            .width(Length::Fill)
            .style(|_theme: &iced::Theme, status| button::Style {
                background: Some(iced::Background::Color(
                    if status == button::Status::Hovered {
                        theme::BG_SLATE_700
                    } else {
                        theme::BG_SLATE_850
                    }
                )),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding(10)
            .into();

            let menu_container = container(
                column![sync_item]
                    .spacing(5)
                    .width(Length::Fixed(180.0))
            )
            .padding(5)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(theme::BG_SLATE_850)),
                border: iced::Border {
                    color: theme::BG_SLATE_700,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

            row![space_fill_x(), menu_container]
                .width(Length::Fill)
                .into()
        } else {
            space_y(0.0).into()
        };

        let metadata_column = column![
            title_row,
            menu_dropdown,
            text(&self.base.author).size(18).color(theme::TEXT_SLATE_300),
            space_y(10.0),
            text(format!("Chapters Count: {}", self.chapters_count)).color(theme::TEXT_SLATE_400),
            text(format!("Rating: {:.1} ⭐", self.base.rating)).color(theme::TEXT_SLATE_300),
            text(format!("Status: {}", self.base.status)).color(theme::TEXT_SLATE_400),
            text(format!("Genres: {}", self.base.genres.join(", "))).color(theme::TEXT_SLATE_400),
            space_y(5.0),
            action_row,
        ]
        .spacing(8)
        .width(Length::Fill);

        let main_info = row![cover_element, metadata_column]
            .spacing(25)
            .width(Length::Fill)
            .align_y(Alignment::Start);

        let mut content = column![
            main_info,
            space_y(20.0),
            bold_text("Summary").size(20).color(theme::TEXT_SLATE_50),
            space_y(5.0),
            text(&self.base.summary).color(theme::TEXT_SLATE_300),
            space_y(20.0),
            bold_text("Chapters").size(20).color(theme::TEXT_SLATE_50),
            space_y(10.0),
        ]
        .spacing(10);

        for chapter in &self.chapters {
            let read_indicator = if chapter.last_read > 0 {
                text("✔ Read").size(11).color(theme::TEXT_SLATE_400)
            } else {
                text("").size(11)
            };

            let chapter_btn = button(
                container(
                    row![
                        text(format!("- {}", chapter.title)).color(theme::TEXT_SLATE_300),
                        space_fill_x(),
                        read_indicator,
                    ]
                    .align_y(Alignment::Center)
                )
                .padding(8)
            )
            .on_press(Message::LoadChapter(chapter.id.clone(), chapter.title.clone(), false))
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(
                    if status == button::Status::Hovered {
                        theme::BG_SLATE_700
                    } else {
                        theme::BG_SLATE_800
                    }
                )),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            content = content.push(chapter_btn);
        }

        scrollable(content.padding(10)).into()
    }
}
