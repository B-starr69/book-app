use crate::app::BookApp;
use crate::cover_cache::cache_cover_sync;
use crate::state::{Command, StreamingState, View};
use crate::{
    ACCENT_COLOR, CARD_BG_DARK, CARD_BG_LIGHT, SIDEBAR_BG_DARK, SIDEBAR_BG_LIGHT, SUCCESS_COLOR,
    WARNING_COLOR,
};
use book_core::{Book, BookPreview, SearchResult};
use eframe::{egui, App, Frame};
use egui::{CentralPanel, Color32, Context, FontId, RichText, TopBottomPanel, Vec2};

impl BookApp {
    pub fn render_bottom_nav(&mut self, ctx: &Context) {
        let bg_color = if self.dark_mode {
            SIDEBAR_BG_DARK
        } else {
            SIDEBAR_BG_LIGHT
        };

        TopBottomPanel::bottom("bottom_nav")
            .exact_height(56.0)
            .frame(egui::Frame::new().fill(bg_color).inner_margin(Vec2::new(4.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let item_width = ui.available_width() / 4.0;

                    self.nav_item_mobile(ui, "Library", View::Library, ctx, item_width);
                    self.nav_item_mobile(ui, "Discover", View::Discover, ctx, item_width);
                    self.nav_item_mobile(ui, "Search", View::Search, ctx, item_width);
                    self.nav_item_mobile(ui, "Settings", View::Settings, ctx, item_width);
                });
            });
    }

    fn nav_item_mobile(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        target_view: View,
        ctx: &Context,
        width: f32,
    ) {
        let is_active =
            std::mem::discriminant(&self.current_view) == std::mem::discriminant(&target_view);
        let text_color = if is_active {
            ACCENT_COLOR
        } else if self.dark_mode {
            Color32::LIGHT_GRAY
        } else {
            Color32::DARK_GRAY
        };
        let bg_color = if is_active {
            ACCENT_COLOR.linear_multiply(0.15)
        } else {
            Color32::TRANSPARENT
        };

        let response = ui.allocate_ui_with_layout(
            Vec2::new(width, 56.0), // Taller nav bar
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 56.0), egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    // Active background indicator (pill shape)
                    if is_active {
                        let indicator_rect = egui::Rect::from_center_size(
                            egui::pos2(rect.center().x, rect.center().y),
                            Vec2::new(64.0, 32.0)
                        );
                        ui.painter().rect_filled(indicator_rect, 16.0, bg_color);
                    }

                    let center = rect.center();
                    ui.painter().text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(14.0), // Larger text
                        text_color,
                    );
                }
                response
            },
        ).inner;

        if response.clicked() {
            self.current_view = target_view.clone();
            if matches!(target_view, View::Discover) && self.get_discover_sections().is_empty() {
                self.load_discover(ctx);
            }
        }
    }

    pub fn render_library(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        let library_books = self.get_library_books();
        let is_loading = self.is_loading();

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("My Library").size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Refresh button
                if ui
                    .add_enabled(
                        !is_loading && !library_books.is_empty(),
                        egui::Button::new("Refresh")
                            .corner_radius(egui::CornerRadius::same(4))
                            .min_size(Vec2::new(0.0, 40.0)),
                    )
                    .clicked()
                {
                    self.refresh_library_books(ctx);
                }
                ui.label(
                    RichText::new(format!("{} books", library_books.len()))
                        .size(12.0)
                        .color(Color32::GRAY),
                );
            });
        });
        ui.add_space(8.0);

        if is_loading {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Updating library...")
                        .size(12.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        if library_books.is_empty() {
            // Empty state
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("[Book]").size(48.0));
                ui.add_space(12.0);
                ui.label(RichText::new("Your library is empty").size(16.0).strong());
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Browse 'Discover' to find books!")
                        .size(12.0)
                        .color(Color32::GRAY),
                );
            });
        } else {
            // Collect clicks to process after rendering
            let mut clicked_book: Option<Book> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
                let card_bg = if self.dark_mode {
                    CARD_BG_DARK
                } else {
                    CARD_BG_LIGHT
                };

                // Grid layout for books - mobile optimized
                let available_width = ui.available_width();
                let card_width = 110.0; // Fixed width for consistent cards
                let spacing = 16.0;
                let columns = ((available_width + spacing) / (card_width + spacing)).floor() as usize;
                let columns = columns.max(2); // At least 2 columns

                egui::Grid::new("library_grid")
                    .num_columns(columns)
                    .spacing(Vec2::new(spacing, spacing))
                    .show(ui, |ui| {
                        for (i, book) in library_books.iter().enumerate() {
                            if self.render_book_card(ui, book, card_bg) {
                                clicked_book = Some(book.clone());
                            }
                            if (i + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });

            // Handle book click outside the scroll area
            if let Some(book) = clicked_book {
                // Set the source to match the book's source
                let sources = self.get_sources();
                if let Some(source) = sources.iter().find(|s| s.id == book.source_id) {
                    self.change_source(source.clone());
                }
                self.current_view = View::BookDetails(book.id.clone());
                self.load_book_details(book.id, ctx);
            }
        }
    }

    fn render_book_card(&mut self, ui: &mut egui::Ui, book: &Book, card_bg: Color32) -> bool {
        let card_width = 110.0;
        let card_height = 170.0;
        let cover_height = 140.0;

        // Check for cached cover first
        let cover_url = Self::get_cached_cover_url(&book.source_id, &book.id)
            .unwrap_or_else(|| book.cover_url.clone());
        let has_cover = !cover_url.is_empty();

        // Calculate reading progress
        let progress = if !book.chapters.is_empty() {
            let read_chapters = book.chapters.iter().filter(|c| c.progress > 0.5).count();
            read_chapters as f32 / book.chapters.len() as f32
        } else {
            0.0
        };

        let response = egui::Frame::new()
            .fill(card_bg)
            .corner_radius(egui::CornerRadius::same(16)) // Rounder corners
            .shadow(egui::epaint::Shadow {
                spread: 4,
                blur: 12,
                offset: [0, 4],
                color: Color32::from_black_alpha(30),
            })
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(card_width, card_height));

                let rect = ui.available_rect_before_wrap();
                let cover_rect = egui::Rect::from_min_size(rect.min, Vec2::new(card_width, cover_height));

                // Cover image
                let show_image = has_cover && cover_url.starts_with("file://");
                if show_image {
                    ui.put(cover_rect,
                        egui::Image::new(&cover_url)
                            .fit_to_exact_size(Vec2::new(card_width, cover_height))
                            .corner_radius(egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 })
                            .show_loading_spinner(false)
                    );
                } else if has_cover {
                    ui.painter().rect_filled(
                        cover_rect,
                        egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                        ACCENT_COLOR.linear_multiply(0.15)
                    );
                    ui.put(cover_rect, |ui: &mut egui::Ui| {
                        ui.centered_and_justified(|ui| ui.spinner());
                        ui.response()
                    });
                } else {
                    ui.painter().rect_filled(
                        cover_rect,
                        egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                        ACCENT_COLOR.linear_multiply(0.2)
                    );
                    ui.put(cover_rect, |ui: &mut egui::Ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("[No Cover]").size(12.0));
                        });
                        ui.response()
                    });
                }

                // Progress bar at bottom of cover
                if progress > 0.0 {
                    let progress_rect = egui::Rect::from_min_size(
                        egui::pos2(cover_rect.min.x, cover_rect.max.y - 3.0),
                        Vec2::new(card_width * progress, 3.0)
                    );
                    ui.painter().rect_filled(progress_rect, 0, SUCCESS_COLOR);
                }

                // Title area below cover
                let title_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 6.0, cover_rect.max.y + 4.0),
                    Vec2::new(card_width - 12.0, card_height - cover_height - 8.0)
                );

                let display_title = &book.title;
                let max_len = 15;
                let truncated = if display_title.chars().count() > max_len {
                    format!("{}...", display_title.chars().take(max_len).collect::<String>())
                } else {
                    display_title.clone()
                };

                ui.put(title_rect, |ui: &mut egui::Ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.label(RichText::new(truncated).size(11.0).strong());
                    });
                    ui.response()
                });
            });

        // Interaction logic
        let card_response = response.response.interact(egui::Sense::click());
        if card_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        card_response.clicked()
    }

    pub fn render_search_page(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        let is_global_searching = self.is_global_searching();
        let global_search_results = self.get_global_search_results();
        let global_search_streaming = self.get_global_search_streaming_state();
        let error_message = self.get_error_message();

        ui.add_space(4.0);
        ui.label(RichText::new("🌐 Search All").size(18.0).strong());
        ui.add_space(6.0);

        // Search bar - full width for mobile
        ui.horizontal(|ui| {
            let available = ui.available_width() - 80.0;
            let search_response = ui.add_sized(
                Vec2::new(available.max(150.0), 32.0),
                egui::TextEdit::singleline(&mut self.global_search_query)
                    .hint_text("🔍 Search...")
                    .font(FontId::proportional(13.0)),
            );

            let search_clicked = ui
                .add_enabled(
                    !is_global_searching && !self.global_search_query.trim().is_empty(),
                    egui::Button::new(if is_global_searching { "⏳" } else { "🔍" })
                        .corner_radius(egui::CornerRadius::same(6))
                        .min_size(Vec2::new(44.0, 32.0)),
                )
                .clicked();

            // Search on Enter or button click
            if (search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || search_clicked
            {
                if !self.global_search_query.trim().is_empty() {
                    self.perform_global_search(ctx);
                }
            }

            // Clear button
            if !self.global_search_query.is_empty() {
                if ui.button("✕").clicked() {
                    self.global_search_query.clear();
                }
            }
        });
        ui.add_space(8.0);

        // Show spinner only if searching with no results yet (initial load)
        if is_global_searching && global_search_results.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Searching...")
                        .size(14.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        // Error message
        if let Some(err) = error_message {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("⚠").size(36.0).color(WARNING_COLOR));
                ui.add_space(6.0);
                ui.label(RichText::new(&err).size(12.0).color(Color32::RED));
            });
            return;
        }

        let card_bg = if self.dark_mode {
            CARD_BG_DARK
        } else {
            CARD_BG_LIGHT
        };

        // Group results by source
        if !global_search_results.is_empty() {
            // Cache covers for results
            for result in &global_search_results {
                let source_id = result.source_id.as_deref().unwrap_or("");
                let cache_key = format!("{}_{}", source_id, result.id);
                if !result.cover_url.is_empty()
                    && Self::get_cached_cover_url(source_id, &result.id).is_none()
                    && !self.pending_cover_downloads.contains(&cache_key)
                {
                    self.pending_cover_downloads.insert(cache_key);
                    let sid = source_id.to_string();
                    let bid = result.id.clone();
                    let curl = result.cover_url.clone();
                    std::thread::spawn(move || {
                        cache_cover_sync(&sid, &bid, &curl);
                    });
                }
            }

            // Group by source name (with owned data)
            let mut grouped: std::collections::HashMap<String, Vec<SearchResult>> =
                std::collections::HashMap::new();
            for result in global_search_results {
                let source_name = result
                    .source_name
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                grouped.entry(source_name).or_default().push(result);
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (source_name, results) in &grouped {
                    // Section header - mobile sized
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(source_name)
                                .size(14.0)
                                .strong()
                                .color(ACCENT_COLOR),
                        );
                        ui.label(
                            RichText::new(format!("({})", results.len()))
                                .size(10.0)
                                .color(Color32::GRAY),
                        );
                    });
                    ui.add_space(6.0);

                    // Horizontal scroll for results
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("search_{}", source_name))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for result in results {
                                    self.render_global_search_card(ui, result, card_bg, ctx);
                                }
                            });
                        });

                    ui.add_space(12.0);
                }

                // Show streaming progress indicator at bottom
                if global_search_streaming.is_loading() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.add_space(8.0);
                        if let StreamingState::Loading { items_loaded } = global_search_streaming {
                            ui.label(
                                RichText::new(format!("Searching more sources... ({} done)", items_loaded))
                                    .size(12.0)
                                    .color(Color32::GRAY),
                            );
                        }
                    });
                    ui.add_space(16.0);
                }
            });
        } else if !self.global_search_query.is_empty() && !is_global_searching {
            // Show "no results" message if search was performed
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("No results found")
                        .size(14.0)
                        .color(Color32::GRAY),
                );
            });
        } else {
            // Initial state - prompt to search
            ui.vertical_centered(|ui| {
                let sources = self.get_sources();
                ui.add_space(60.0);
                ui.label(RichText::new("🔍").size(48.0));
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Search all sources")
                        .size(14.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} sources",
                        sources.iter().filter(|s| s.config.search.is_some()).count()
                    ))
                    .size(11.0)
                    .color(Color32::DARK_GRAY),
                );
            });
        }
    }

    fn render_global_search_card(
        &mut self,
        ui: &mut egui::Ui,
        result: &SearchResult,
        card_bg: Color32,
        ctx: &Context,
    ) {
        let card_width = 110.0;
        let card_height = 180.0;
        let cover_height = 140.0;

        // Use source_id from result
        let source_id = result.source_id.as_deref().unwrap_or("");
        let source_name = result.source_name.as_deref().unwrap_or("");

        // Check if book is in library
        let library_books = self.get_library_books();
        let is_in_library = library_books
            .iter()
            .any(|b| b.id == result.id && b.source_id == source_id);

        // Check for cached cover
        let cover_url = Self::get_cached_cover_url(source_id, &result.id)
            .unwrap_or_else(|| result.cover_url.clone());
        let has_cover = !cover_url.is_empty();

        let response = egui::Frame::new()
            .fill(card_bg)
            .corner_radius(egui::CornerRadius::same(12))
            .shadow(egui::epaint::Shadow {
                spread: 2,
                blur: 8,
                offset: [0, 2],
                color: Color32::from_black_alpha(40),
            })
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(card_width, card_height));

                let rect = ui.available_rect_before_wrap();
                let cover_rect = egui::Rect::from_min_size(rect.min, Vec2::new(card_width, cover_height));

                // Cover image
                let show_image = has_cover && cover_url.starts_with("file://");
                if show_image {
                    ui.put(cover_rect,
                        egui::Image::new(&cover_url)
                            .fit_to_exact_size(Vec2::new(card_width, cover_height))
                            .corner_radius(egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 })
                            .show_loading_spinner(false)
                    );
                } else if has_cover {
                    ui.painter().rect_filled(
                        cover_rect,
                        egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                        ACCENT_COLOR.linear_multiply(0.15)
                    );
                    ui.put(cover_rect, |ui: &mut egui::Ui| {
                        ui.centered_and_justified(|ui| ui.spinner());
                        ui.response()
                    });
                } else {
                    ui.painter().rect_filled(
                        cover_rect,
                        egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                        ACCENT_COLOR.linear_multiply(0.2)
                    );
                    ui.put(cover_rect, |ui: &mut egui::Ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("[No Cover]").size(12.0));
                        });
                        ui.response()
                    });
                }

                // Library indicator badge (top-right corner)
                if is_in_library {
                    let badge_pos = egui::pos2(cover_rect.max.x - 24.0, cover_rect.min.y + 6.0);
                    let badge_rect = egui::Rect::from_min_size(badge_pos, Vec2::new(18.0, 18.0));
                    ui.painter().rect_filled(badge_rect, 9, SUCCESS_COLOR);
                    ui.painter().text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(10.0),
                        Color32::WHITE
                    );
                }

                // Source badge (top-left corner)
                if !source_name.is_empty() {
                    let badge_pos = egui::pos2(cover_rect.min.x + 4.0, cover_rect.min.y + 4.0);
                    let text = &source_name[..source_name.len().min(8)];
                    let badge_rect = egui::Rect::from_min_size(badge_pos, Vec2::new(text.len() as f32 * 5.5 + 8.0, 16.0));
                    ui.painter().rect_filled(badge_rect, 4, Color32::from_black_alpha(160));
                    ui.painter().text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional(9.0),
                        Color32::WHITE
                    );
                }

                // Title area
                let display_title = if !result.title.is_empty() {
                    result.title.clone()
                } else {
                    result.id.split('-')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                                None => String::new(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                let title_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 6.0, cover_rect.max.y + 4.0),
                    Vec2::new(card_width - 12.0, card_height - cover_height - 8.0)
                );

                let max_len = 15;
                let truncated = if display_title.chars().count() > max_len {
                    format!("{}...", display_title.chars().take(max_len).collect::<String>())
                } else {
                    display_title
                };

                ui.put(title_rect, |ui: &mut egui::Ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(truncated).size(11.0).strong());
                        if let Some(count) = result.chapters_count {
                            if count > 0 {
                                ui.label(RichText::new(format!("{} ch", count)).size(9.0).color(Color32::GRAY));
                            }
                        }
                    });
                    ui.response()
                });
            });

        // Make entire card clickable
        if response.response.interact(egui::Sense::click()).clicked() {
            let sources = self.get_sources();
            if let Some(source) = sources.iter().find(|s| s.id == source_id) {
                self.change_source(source.clone());
            }
            self.current_view = View::BookDetails(result.id.clone());
            self.load_book_details(result.id.clone(), ctx);
        }

        if response.response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    pub fn render_settings(&mut self, _ctx: &Context, ui: &mut egui::Ui) {
        let sources = self.get_sources();
        let card_bg = if self.dark_mode { CARD_BG_DARK } else { CARD_BG_LIGHT };
        let text_color = if self.dark_mode { Color32::WHITE } else { Color32::BLACK };

        ui.add_space(8.0);
        ui.label(RichText::new("Settings").size(20.0).strong().color(text_color));
        ui.add_space(16.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Sources section
            ui.label(RichText::new("Sources").size(16.0).strong().color(text_color));
            ui.add_space(8.0);

            // List current sources
            let mut source_to_delete: Option<String> = None;
            for source in &sources {
                egui::Frame::new()
                    .fill(card_bg)
                    .corner_radius(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&source.name).size(14.0).strong().color(text_color));
                                ui.label(RichText::new(&source.url).size(12.0).color(Color32::GRAY));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(egui::Button::new(RichText::new("🗑").size(16.0))
                                        .min_size(Vec2::new(44.0, 44.0)))
                                    .clicked()
                                {
                                    source_to_delete = Some(source.id.clone());
                                }
                            });
                        });
                    });
                ui.add_space(8.0);
            }

            // Delete source if requested
            if let Some(source_id) = source_to_delete {
                self.delete_source(source_id);
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(16.0);

            // Add new source section
            ui.label(RichText::new("Add New Source").size(16.0).strong().color(text_color));
            ui.add_space(8.0);
            ui.label(RichText::new("Paste source configuration JSON:").size(12.0).color(Color32::GRAY));
            ui.add_space(4.0);

            // JSON input area
            egui::Frame::new()
                .fill(card_bg)
                .corner_radius(8.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.source_json_input)
                                    .desired_width(f32::INFINITY)
                                    .min_size(Vec2::new(0.0, 150.0))
                                    .font(FontId::monospace(12.0))
                                    .hint_text("Paste JSON configuration here...")
                            );
                        });
                });

            ui.add_space(8.0);

            // Error message
            if let Some(ref err) = self.source_error {
                ui.label(RichText::new(err).size(12.0).color(WARNING_COLOR));
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(RichText::new("Add Source").size(14.0))
                        .min_size(Vec2::new(0.0, 44.0)))
                    .clicked()
                {
                    // Try to parse JSON
                    match serde_json::from_str::<book_core::SourceWithConfig>(&self.source_json_input) {
                        Ok(source) => {
                            self.save_source(source);
                            self.source_json_input.clear();
                            self.source_error = None;
                        }
                        Err(e) => {
                            self.source_error = Some(format!("Invalid JSON: {}", e));
                        }
                    }
                }
                if ui
                    .add(egui::Button::new(RichText::new("Load Example").size(14.0))
                        .min_size(Vec2::new(0.0, 44.0)))
                    .clicked()
                {
                    // Load example template
                    let example = book_core::defaults::example_source_template();
                    self.source_json_input = serde_json::to_string_pretty(&example).unwrap_or_default();
                    self.source_error = None;
                }
            });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);

            // Appearance section
            ui.label(RichText::new("Appearance").size(16.0).strong().color(text_color));
            ui.add_space(8.0);

            egui::Frame::new()
                .fill(card_bg)
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Theme:").color(text_color));
                        ui.add_space(16.0);
                        if ui.selectable_label(self.dark_mode, "🌙 Dark").clicked() {
                            self.dark_mode = true;
                        }
                        ui.add_space(8.0);
                        if ui.selectable_label(!self.dark_mode, "☀ Light").clicked() {
                            self.dark_mode = false;
                        }
                    });

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Reader Font Size:").color(text_color));
                        ui.add(egui::Slider::new(&mut self.reader_font_size, 12.0..=32.0).suffix("px"));
                    });
                });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);

            // Cache section
            ui.label(RichText::new("Storage").size(16.0).strong().color(text_color));
            ui.add_space(8.0);

            egui::Frame::new()
                .fill(card_bg)
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Covers Cache:").color(text_color));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new(RichText::new("Clear Covers").size(12.0))
                                    .min_size(Vec2::new(0.0, 44.0)))
                                .clicked()
                            {
                                // Clear covers directory
                                let _ = std::fs::remove_dir_all("covers");
                                let _ = std::fs::create_dir_all("covers");
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Database:").color(text_color));
                        ui.label(RichText::new("library.db").size(12.0).color(Color32::GRAY));
                    });
                });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);

            // About section
            ui.label(RichText::new("About").size(16.0).strong().color(text_color));
            ui.add_space(8.0);

            egui::Frame::new()
                .fill(card_bg)
                .corner_radius(8.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Book Reader").size(14.0).strong().color(text_color));
                    ui.label(RichText::new("Version 0.1.0").size(12.0).color(Color32::GRAY));
                    ui.add_space(8.0);
                    ui.label(RichText::new("A novel reader with configurable sources.").size(12.0).color(Color32::GRAY));
                });

            ui.add_space(32.0);
        });
    }

    pub fn render_discover(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        let sources = self.get_sources();
        let selected_source = self.get_selected_source();
        let is_loading = self.is_loading();
        let is_searching = self.is_searching();
        let error_message = self.get_error_message();
        let search_results = self.get_search_results();
        let discover_sections = self.get_discover_sections();
        let discover_streaming = self.get_discover_streaming_state();

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Discover").size(18.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("Refresh")
                        .corner_radius(egui::CornerRadius::same(6))
                        .min_size(Vec2::new(0.0, 36.0)))
                    .clicked()
                {
                    self.search_query.clear();
                    self.load_discover(ctx);
                }
            });
        });
        ui.add_space(4.0);

        // Source tabs (horizontal scrollable for mobile)
        egui::ScrollArea::horizontal()
            .id_salt("source_tabs")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for source in &sources {
                        let is_selected =
                            selected_source.as_ref().map(|s| &s.id) == Some(&source.id);

                        let btn = if is_selected {
                            egui::Button::new(
                                RichText::new(&source.name)
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(ACCENT_COLOR)
                            .corner_radius(egui::CornerRadius::same(6))
                            .min_size(Vec2::new(0.0, 40.0))
                        } else {
                            let text_color = if self.dark_mode {
                                Color32::LIGHT_GRAY
                            } else {
                                Color32::DARK_GRAY
                            };
                            egui::Button::new(RichText::new(&source.name).size(13.0).color(text_color))
                                .fill(Color32::TRANSPARENT)
                                .corner_radius(egui::CornerRadius::same(6))
                                .min_size(Vec2::new(0.0, 40.0))
                        };

                        if ui.add(btn).clicked() && !is_selected {
                            self.change_source(source.clone());
                            self.search_query.clear();
                            self.load_discover(ctx);
                        }
                        ui.add_space(2.0);
                    }
                });
            });
        ui.add_space(6.0);

        // Search bar - mobile optimized
        let has_search = selected_source
            .as_ref()
            .and_then(|s| s.config.search.as_ref())
            .is_some();

        if has_search {
            ui.add_space(8.0);
            let search_response = egui::Frame::new()
                .fill(if self.dark_mode {
                    Color32::from_gray(35)
                } else {
                    Color32::from_gray(245)
                })
                .corner_radius(egui::CornerRadius::same(24))
                .inner_margin(Vec2::new(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔍").size(16.0));
                        ui.add_sized(
                            ui.available_size(),
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Search novels...")
                                .frame(false)
                                .font(FontId::proportional(16.0))
                        );
                    });
                });

            if search_response.response.clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                 if !self.search_query.trim().is_empty() {
                    self.perform_search(ctx);
                 }
            }
        }
        ui.add_space(8.0);

        // Show spinner only if loading with no sections yet (initial load)
        // Or if searching (search is not streamed yet)
        if is_searching {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(RichText::new("Searching...").size(12.0).color(Color32::GRAY));
            });
            return;
        }

        // Show initial loading spinner only if no sections loaded yet
        if is_loading && discover_sections.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(RichText::new("Loading...").size(12.0).color(Color32::GRAY));
            });
            return;
        }

        if let Some(err) = error_message {
            let should_retry = ui
                .vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("⚠").size(36.0).color(WARNING_COLOR));
                    ui.add_space(6.0);
                    ui.label(RichText::new(&err).size(12.0).color(Color32::RED));
                    ui.add_space(12.0);
                    ui.add(egui::Button::new("Refresh")
                        .corner_radius(egui::CornerRadius::same(4))
                        .min_size(Vec2::new(0.0, 44.0)))
                        .clicked()
                })
                .inner;
            if should_retry {
                self.load_discover(ctx);
            }
            return;
        }

        let card_bg = if self.dark_mode {
            CARD_BG_DARK
        } else {
            CARD_BG_LIGHT
        };

        // Show search results if available, otherwise show discover sections
        if !search_results.is_empty() {
            self.render_search_results(ui, card_bg, ctx);
        } else {
            // Trigger background cover downloads for discover sections (only first visible ones)
            let source_id = selected_source
                .as_ref()
                .map(|s| s.id.clone())
                .unwrap_or_default();
            // Limit cover downloads to first 3 sections to reduce thread spawn overhead
            for section in discover_sections.iter().take(3) {
                for book in section.books.iter().take(20) {
                    let cache_key = format!("{}_{}", source_id, book.id);
                    if !book.cover_url.is_empty()
                        && Self::get_cached_cover_url(&source_id, &book.id).is_none()
                        && !self.pending_cover_downloads.contains(&cache_key)
                    {
                        self.pending_cover_downloads.insert(cache_key);
                        let sid = source_id.clone();
                        let bid = book.id.clone();
                        let curl = book.cover_url.clone();
                        std::thread::spawn(move || {
                            cache_cover_sync(&sid, &bid, &curl);
                        });
                    }
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for section in &discover_sections {
                    // Section header - mobile sized
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&section.title).size(14.0).strong());
                        ui.label(
                            RichText::new(format!("({})", section.books.len()))
                                .size(10.0)
                                .color(Color32::GRAY),
                        );
                    });
                    ui.add_space(6.0);

                    // Horizontal scroll for books in section
                    // Limit to first 20 books per section to reduce image loading lag
                    let visible_books: Vec<_> = section.books.iter().take(20).collect();
                    egui::ScrollArea::horizontal()
                        .id_salt(&section.title)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for book_preview in visible_books {
                                    self.render_discover_card(ui, book_preview, card_bg, ctx);
                                }
                            });
                        });

                    ui.add_space(12.0);
                }

                // Show streaming progress indicator at bottom
                if discover_streaming.is_loading() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.add_space(8.0);
                        if let StreamingState::Loading { items_loaded } = discover_streaming {
                            ui.label(
                                RichText::new(format!("Loading more sections... ({} loaded)", items_loaded))
                                    .size(12.0)
                                    .color(Color32::GRAY),
                            );
                        }
                    });
                    ui.add_space(16.0);
                }
            });
        }
    }

    fn render_search_results(&mut self, ui: &mut egui::Ui, card_bg: Color32, ctx: &Context) {
        let search_results = self.get_search_results();
        let selected_source = self.get_selected_source();

        ui.horizontal(|ui| {
            ui.label(RichText::new("Results").size(14.0).strong());
            ui.label(
                RichText::new(format!("({})", search_results.len()))
                    .size(10.0)
                    .color(Color32::GRAY),
            );
        });
        ui.add_space(6.0);

        // Convert SearchResult to BookPreview and reuse existing card rendering
        let book_previews: Vec<BookPreview> = search_results
            .iter()
            .map(|r| BookPreview {
                id: r.id.clone(),
                title: r.title.clone(),
                cover_url: r.cover_url.clone(),
            })
            .collect();

        // Cache covers for search results (in background, avoiding duplicates)
        let source_id = selected_source
            .as_ref()
            .map(|s| s.id.clone())
            .unwrap_or_default();
        for result in &search_results {
            let cache_key = format!("{}_{}", source_id, result.id);
            if !result.cover_url.is_empty()
                && Self::get_cached_cover_url(&source_id, &result.id).is_none()
                && !self.pending_cover_downloads.contains(&cache_key)
            {
                self.pending_cover_downloads.insert(cache_key);
                let sid = source_id.clone();
                let bid = result.id.clone();
                let curl = result.cover_url.clone();
                std::thread::spawn(move || {
                    cache_cover_sync(&sid, &bid, &curl);
                });
            }
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Horizontal scroll layout matching discover sections
            egui::ScrollArea::horizontal()
                .id_salt("search_results")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for book_preview in &book_previews {
                            self.render_discover_card(ui, book_preview, card_bg, ctx);
                        }
                    });
                });
        });
    }

    fn render_discover_card(
        &mut self,
        ui: &mut egui::Ui,
        book_preview: &BookPreview,
        card_bg: Color32,
        ctx: &Context,
    ) {
        let card_width = 110.0;
        let card_height = 170.0;
        let cover_height = 140.0;

        // Check if book is in library
        let library_books = self.get_library_books();
        let is_in_library = library_books.iter().any(|b| b.id == book_preview.id);

        // Check for cached cover (using selected source id)
        let selected_source = self.get_selected_source();
        let source_id = selected_source.as_ref().map(|s| s.id.as_str()).unwrap_or("");
        let cover_url = Self::get_cached_cover_url(source_id, &book_preview.id)
            .unwrap_or_else(|| book_preview.cover_url.clone());
        let has_cover = !cover_url.is_empty();

        let response = egui::Frame::new()
            .fill(card_bg)
            .corner_radius(egui::CornerRadius::same(12))
            .shadow(egui::epaint::Shadow {
                spread: 0,
                blur: 3,
                offset: [0, 1],
                color: Color32::from_black_alpha(15),
            })
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(card_width, card_height));

                let rect = ui.available_rect_before_wrap();
                let cover_rect = egui::Rect::from_min_size(rect.min, Vec2::new(card_width, cover_height));

                // Cover image
                let show_image = has_cover && cover_url.starts_with("file://");
                if show_image {
                    ui.put(cover_rect,
                        egui::Image::new(&cover_url)
                            .fit_to_exact_size(Vec2::new(card_width, cover_height))
                            .corner_radius(egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 })
                            .show_loading_spinner(false)
                    );
                } else if has_cover {
                    ui.painter().rect_filled(
                        cover_rect,
                        egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                        ACCENT_COLOR.linear_multiply(0.15)
                    );
                    // Skip spinner on placeholder - just show tinted background to reduce render overhead
                } else {
                    ui.painter().rect_filled(
                        cover_rect,
                        egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                        ACCENT_COLOR.linear_multiply(0.2)
                    );
                    ui.put(cover_rect, |ui: &mut egui::Ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("[No Cover]").size(12.0));
                        });
                        ui.response()
                    });
                }

                // Library indicator badge (top-right corner)
                if is_in_library {
                    let badge_pos = egui::pos2(cover_rect.max.x - 24.0, cover_rect.min.y + 6.0);
                    let badge_rect = egui::Rect::from_min_size(badge_pos, Vec2::new(18.0, 18.0));
                    ui.painter().rect_filled(badge_rect, 9, SUCCESS_COLOR);
                    ui.painter().text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(10.0),
                        Color32::WHITE
                    );
                }

                // Title area
                let display_title = if !book_preview.title.is_empty() {
                    book_preview.title.clone()
                } else {
                    book_preview.id.split('-')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                                None => String::new(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                let title_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 6.0, cover_rect.max.y + 4.0),
                    Vec2::new(card_width - 12.0, card_height - cover_height - 8.0)
                );

                let max_len = 15;
                let truncated = if display_title.chars().count() > max_len {
                    format!("{}...", display_title.chars().take(max_len).collect::<String>())
                } else {
                    display_title
                };

                ui.put(title_rect, |ui: &mut egui::Ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.label(RichText::new(truncated).size(11.0).strong());
                    });
                    ui.response()
                });
            });

        // Make entire card clickable
        if response.response.interact(egui::Sense::click()).clicked() {
            self.current_view = View::BookDetails(book_preview.id.clone());
            self.load_book_details(book_preview.id.clone(), ctx);
        }

        // Hover effect
        if response.response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    pub fn render_book_details(&mut self, ctx: &Context, ui: &mut egui::Ui, _book_id: &str) {
        let book_details_streaming = self.get_book_details_streaming_state();

        // Show loading spinner only if no book data yet
        if self.is_loading() && self.get_current_book().is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Loading...")
                        .size(12.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        if let Some(book) = self.get_current_book() {
            let card_bg = if self.dark_mode {
                CARD_BG_DARK
            } else {
                CARD_BG_LIGHT
            };

            // Header with back button and refresh
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("⬅ Back")
                        .frame(false)
                        .min_size(Vec2::new(0.0, 44.0)))
                    .clicked()
                {
                    if book.in_library {
                        self.current_view = View::Library;
                    } else {
                        self.current_view = View::Discover;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new("↻").min_size(Vec2::new(44.0, 44.0))).clicked() {
                        self.refresh_current_book(ctx);
                    }
                });
            });
            ui.add_space(8.0);

            let _scroll_output = egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    // Hero Section
                    let hero_height = 220.0;
                    let (hero_rect, _hero_response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), hero_height),
                        egui::Sense::hover()
                    );

                    // Background
                    let cover_url = Self::get_cached_cover_url(&book.source_id, &book.id)
                        .unwrap_or_else(|| book.cover_url.clone());

                    if ui.is_rect_visible(hero_rect) {
                         // Dark background
                        ui.painter().rect_filled(hero_rect, 0.0, if self.dark_mode {
                            Color32::from_black_alpha(200)
                        } else {
                            Color32::from_gray(240)
                        });

                        // Cover Foreground
                        let cover_width = 120.0;
                        let cover_height = 180.0;
                        let cover_rect = egui::Rect::from_center_size(
                            hero_rect.center() - Vec2::new(0.0, 10.0),
                            Vec2::new(cover_width, cover_height)
                        );

                        if !cover_url.is_empty() {
                            // Shadow
                            ui.painter().add(egui::epaint::Shadow {
                                 spread: 8,
                                 blur: 20,
                                 offset: [0, 8],
                                 color: Color32::from_black_alpha(100),
                            }.as_shape(cover_rect, egui::CornerRadius::same(8)));

                            // Image
                            ui.put(cover_rect,
                                egui::Image::new(&cover_url)
                                    .fit_to_exact_size(Vec2::new(cover_width, cover_height))
                                    .corner_radius(egui::CornerRadius::same(8))
                            );
                        } else {
                            ui.painter().rect_filled(cover_rect, 8.0, ACCENT_COLOR);
                        }
                    }

                    // Metadata below hero
                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(&book.title).size(20.0).strong());
                        ui.add_space(4.0);
                        ui.label(RichText::new(&book.author).size(14.0).color(Color32::GRAY));
                    });

                    ui.add_space(16.0);

                    // Genre chips
                    ui.horizontal_wrapped(|ui| {
                         ui.spacing_mut().item_spacing.x = 8.0;
                         ui.spacing_mut().item_spacing.y = 8.0;
                         // Center align hack-ish
                         let total_width: f32 = book.genres.iter().map(|g| g.len() as f32 * 7.0 + 16.0).sum();
                         let margin = (ui.available_width() - total_width).max(0.0) / 2.0;
                         ui.add_space(margin);

                         for genre in &book.genres {
                            egui::Frame::new()
                                .fill(ui.visuals().faint_bg_color)
                                .corner_radius(egui::CornerRadius::same(12))
                                .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_fill))
                                .inner_margin(Vec2::new(10.0, 5.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(genre).size(10.0));
                                });
                             }
                    });

                    ui.add_space(8.0);

                    // Action buttons row
                    ui.horizontal(|ui| {
                        // Read button - determine where to continue reading
                        let has_chapters = !book.chapters.is_empty();

                        // Find the chapter to read:
                        // Sequential progress: Find the first chapter that isn't fully read (progress < 1.0)
                        // If all chapters are read, go to the last chapter
                        let resume_chapter = if has_chapters {
                            // Find first chapter not fully read
                            let first_incomplete = book.chapters.iter()
                                .find(|c| c.progress < 1.0);

                            if let Some(ch) = first_incomplete {
                                Some(ch.id.clone())
                            } else {
                                // All read - go to last chapter
                                book.chapters.last().map(|c| c.id.clone())
                            }
                        } else {
                            None
                        };

                        // Determine button text based on progress
                        let has_progress = book.chapters.iter().any(|c| c.progress > 0.0);
                        let read_btn_text = if has_progress {
                            "Continue"
                        } else {
                            "Read"
                        };

                        if ui
                            .add_enabled(
                                has_chapters,
                                egui::Button::new(
                                    RichText::new(read_btn_text).size(13.0).color(Color32::WHITE),
                                )
                                .fill(ACCENT_COLOR)
                                .corner_radius(egui::CornerRadius::same(6))
                                .min_size(Vec2::new(0.0, 44.0)),
                            )
                            .clicked()
                        {
                            if let Some(chapter_id) = resume_chapter {
                                let book_id = book.id.clone();
                                self.current_view = View::Reader(book_id.clone(), chapter_id.clone());
                                self.load_chapter(book_id, chapter_id, ctx);
                            }
                        }

                        ui.add_space(8.0);

                        // Add to library button
                        let lib_btn_text = if book.in_library {
                            "[x] In Library"
                        } else {
                            "+ Library"
                        };
                        let lib_btn_color = if book.in_library {
                            SUCCESS_COLOR
                        } else {
                            Color32::from_rgb(100, 100, 100)
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(lib_btn_text).size(13.0).color(Color32::WHITE),
                                )
                                .fill(lib_btn_color)
                                .corner_radius(egui::CornerRadius::same(6))
                                .min_size(Vec2::new(0.0, 44.0)),
                            )
                            .clicked()
                        {
                            if book.in_library {
                                self.remove_from_library(&book.id, &book.source_id);
                            } else {
                                self.add_to_library(&book);
                            }
                        }
                    });

                    ui.add_space(12.0);

                    // Summary section - collapsible
                    let summary_header = if self.summary_expanded {
                        "[-] Summary"
                    } else {
                        "[+] Summary"
                    };

                    egui::Frame::new()
                        .fill(card_bg)
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let header_response = ui.add(
                                egui::Label::new(
                                    RichText::new(summary_header).size(12.0).strong(),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if header_response.clicked() {
                                self.summary_expanded = !self.summary_expanded;
                            }
                            if header_response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if self.summary_expanded {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(&book.summary).size(10.0).color(Color32::GRAY),
                                );
                            } else {
                                // Show truncated preview
                                let preview: String = book.summary.chars().take(80).collect();
                                let preview = if book.summary.chars().count() > 80 {
                                    format!("{}...", preview)
                                } else {
                                    preview
                                };
                                ui.add_space(2.0);
                                ui.label(RichText::new(preview).size(9.0).color(Color32::GRAY));
                            }
                        });

                    ui.add_space(12.0);

                    // Chapters section with virtual scrolling for performance
                    let is_chapters_loading = book_details_streaming.is_loading();
                    let loaded_chapters = book.chapters.len();
                    let expected_chapters = book.chapters_count as usize;

                    ui.horizontal(|ui| {
                        if is_chapters_loading {
                            ui.label(
                                RichText::new(format!("Chapters ({}/{})", loaded_chapters, expected_chapters))
                                    .size(14.0)
                                    .strong(),
                            );
                            ui.spinner();
                        } else {
                            ui.label(
                                RichText::new(format!("Chapters ({})", loaded_chapters))
                                    .size(14.0)
                                    .strong(),
                            );
                        }
                    });
                    ui.add_space(6.0);

                    // Use nested ScrollArea with show_rows for virtual scrolling
                    let row_height = 32.0;
                    let total_chapters = book.chapters.len();
                    let chapters = book.chapters.clone();
                    let book_id = book.id.clone();
                    let dark_mode = self.dark_mode;

                    // Show placeholder if no chapters yet and still loading
                    if total_chapters == 0 && is_chapters_loading {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.spinner();
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Loading chapters...")
                                    .size(11.0)
                                    .color(Color32::GRAY),
                            );
                        });
                    } else {
                        // Calculate available height for chapters (leave space for other content)
                        let available_height = ui.available_height().min(500.0).max(200.0);

                        egui::ScrollArea::vertical()
                            .id_salt("chapters_virtual_scroll")
                            .max_height(available_height)
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show_rows(ui, row_height, total_chapters, |ui, row_range| {
                                for i in row_range {
                                    let chapter = &chapters[i];
                                    let is_read = chapter.progress >= 1.0;
                                    let chapter_bg = if i % 2 == 0 {
                                        card_bg
                                    } else {
                                        Color32::TRANSPARENT
                                    };

                                    let response = egui::Frame::new()
                                        .fill(chapter_bg)
                                        .corner_radius(egui::CornerRadius::same(4))
                                        .inner_margin(Vec2::new(8.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.set_min_height(row_height - 8.0);
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                // Read indicator
                                                if is_read {
                                                    ui.label(
                                                        RichText::new("[x]")
                                                            .size(10.0)
                                                            .color(SUCCESS_COLOR),
                                                    );
                                                } else {
                                                    ui.label(
                                                        RichText::new("[ ]")
                                                            .size(10.0)
                                                            .color(Color32::GRAY),
                                                    );
                                                }

                                                let title_color = if is_read {
                                                    Color32::GRAY
                                                } else if dark_mode {
                                                    Color32::WHITE
                                                } else {
                                                    Color32::BLACK
                                                };
                                                // Show full title (truncate only if very long)
                                                let title = &chapter.title;
                                                let max_len = 45;
                                                let truncated = if title.chars().count() > max_len {
                                                    format!(
                                                        "{}...",
                                                        title.chars().take(max_len).collect::<String>()
                                                    )
                                                } else {
                                                    title.clone()
                                                };
                                                ui.label(
                                                    RichText::new(truncated)
                                                        .size(11.0)
                                                        .color(title_color),
                                                );
                                            });
                                        });

                                    if response.response.interact(egui::Sense::click()).clicked() {
                                        self.current_view =
                                            View::Reader(book_id.clone(), chapter.id.clone());
                                        self.load_chapter(book_id.clone(), chapter.id.clone(), ctx);
                                    }

                                    if response.response.hovered() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                }
                            });
                    }
                });
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("Book not found").size(14.0));
                ui.add_space(12.0);
                if ui
                    .add(egui::Button::new("< Back").corner_radius(egui::CornerRadius::same(4)))
                    .clicked()
                {
                    self.current_view = View::Discover;
                }
            });
        }
    }

    pub fn render_reader(&mut self, ui: &mut egui::Ui, book_id: &str, chapter_id: &str, ctx: &Context) {
        // Clear the loading flag if we've switched to displaying this chapter
        if self.currently_loading_chapter_id.as_ref() == Some(&chapter_id.to_string()) {
            self.currently_loading_chapter_id = None;
        }

        if self.is_loading() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.spinner();
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Loading...")
                        .size(12.0)
                        .color(Color32::GRAY),
                );
            });
            return;
        }

        // Track current scroll progress for saving
        let current_progress = self.reader_scroll_offset;
        let show_controls = self.show_reader_controls;

        // Content Area -------------------------------------------------------
        let screen_rect = ui.available_rect_before_wrap();

        // Use a central panel-like area for the text
        let content_rect = screen_rect;

        ui.allocate_ui_at_rect(content_rect, |ui| {
             let scroll_id = egui::Id::new(format!("reader_scroll_{}_{}", book_id, chapter_id));

             // Calculate initial offset if restoring scroll position
            let needs_restore = self.reader_needs_scroll_restore;
            let target_progress = self.reader_target_progress;
            let last_height = self.reader_last_content_height;

            let scroll_area = if needs_restore && last_height > 0.0 && target_progress > 0.0 {
                let viewport_guess = 600.0;
                let max_scroll = (last_height - viewport_guess).max(0.0);
                let target_offset = target_progress * max_scroll;
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
                    .vertical_scroll_offset(target_offset)
            } else {
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
            };

            let scroll_output = scroll_area.show(ui, |ui| {
                // Formatting
                let text_color = if self.dark_mode {
                    Color32::from_gray(220)
                } else {
                    Color32::from_gray(30)
                };

                // Add consistent top padding - controls overlay on top without affecting layout
                ui.add_space(20.0);

                let chapter_content = self.get_chapter_content();
                let plain_text = html_to_plain_text(&chapter_content);

                let font_size = self.reader_font_size;
                let line_spacing = font_size * 0.8;

                // Reader text with margins
                let margin: i8 = 24;
                egui::Frame::new()
                    .inner_margin(egui::Margin { left: margin, right: margin, top: 0, bottom: 0 })
                    .show(ui, |ui| {
                        for paragraph in plain_text.split("\n\n") {
                            let trimmed = paragraph.trim();
                            if !trimmed.is_empty() {
                                ui.label(
                                    RichText::new(trimmed)
                                        .size(font_size)
                                        .color(text_color)
                                        .family(egui::FontFamily::Proportional)
                                        .line_height(Some(font_size + line_spacing)) // Better line height
                                );
                                ui.add_space(line_spacing);
                            }
                        }
                    });

                ui.add_space(40.0);
                ui.interact(ui.min_rect(), ui.id().with("tap"), egui::Sense::click())
            });

            // Handle tap on content to toggle controls (if not dragging)
            if scroll_output.inner.clicked() {
                self.show_reader_controls = !self.show_reader_controls;
            }

            // Calculate progress updates...
             let content_height = scroll_output.content_size.y;
            let viewport_height = scroll_output.inner_rect.height();
            let scroll_offset = scroll_output.state.offset.y;
            self.reader_last_content_height = content_height;

            let max_scroll = (content_height - viewport_height).max(0.0);
            let new_progress = if max_scroll > 0.0 {
                (scroll_offset / max_scroll).clamp(0.0, 1.0)
            } else {
                1.0
            };
            self.reader_scroll_offset = new_progress;

            // Auto-save logic
            let current_time = ui.ctx().input(|i| i.time);
            if current_time - self.last_auto_save > 2.0 {
                 let changed = (new_progress - self.last_saved_progress_value).abs() > 0.001
                    || (new_progress >= 0.99 && self.last_saved_progress_value < 0.99);
                if changed {
                    self.save_chapter_progress(book_id, chapter_id, new_progress);
                    self.last_auto_save = current_time;
                    self.last_saved_progress_value = new_progress;
                }
            }

            if needs_restore {
                self.reader_needs_scroll_restore = false;
            }
        });

        // OVERLAYS -----------------------------------------------------------
        if show_controls {
             // Top Toolbar
             let top_bar_rect = egui::Rect::from_min_size(
                 screen_rect.min,
                 Vec2::new(screen_rect.width(), 56.0)
             );

             ui.painter().rect_filled(top_bar_rect, 0.0, if self.dark_mode {
                 Color32::from_black_alpha(240)
             } else {
                 Color32::from_white_alpha(240)
             });

             ui.allocate_ui_at_rect(top_bar_rect, |ui| {
                 ui.horizontal_centered(|ui| {
                     ui.add_space(8.0);
                     if ui.add(egui::Button::new("⬅").frame(false).min_size(Vec2::new(44.0, 44.0))).clicked() {
                          self.save_chapter_progress(book_id, chapter_id, current_progress);
                          self.current_view = View::BookDetails(book_id.to_string());
                     }

                     // Truncated title
                     let current_book = self.get_current_book();
                     if let Some(book) = current_book {
                          if let Some(chapter) = book.chapters.iter().find(|c| c.id == chapter_id) {
                              ui.label(RichText::new(&chapter.title).strong().size(14.0));
                          }
                     }
                 });
             });

             // Bottom Toolbar
             let bottom_height = 100.0;
             let bottom_bar_rect = egui::Rect::from_min_size(
                 egui::pos2(screen_rect.min.x, screen_rect.max.y - bottom_height),
                 Vec2::new(screen_rect.width(), bottom_height)
             );

              ui.painter().rect_filled(bottom_bar_rect, 0.0, if self.dark_mode {
                 Color32::from_black_alpha(240)
             } else {
                 Color32::from_white_alpha(240)
             });

             ui.allocate_ui_at_rect(bottom_bar_rect, |ui| {
                 ui.vertical_centered(|ui| {
                     ui.add_space(12.0);

                     // Progress Slider
                     let mut progress_val = current_progress;
                     ui.add(egui::Slider::new(&mut progress_val, 0.0..=1.0).show_value(false).text("Progress"));

                     ui.add_space(12.0);

                     // Controls Row
                     ui.horizontal(|ui| {
                         ui.add_space(20.0);

                         // Font Zoom
                         if ui.add(egui::Button::new("A-").min_size(Vec2::new(44.0, 44.0))).clicked() {
                             self.reader_font_size = (self.reader_font_size - 2.0).max(12.0);
                         }
                         if ui.add(egui::Button::new("A+").min_size(Vec2::new(44.0, 44.0))).clicked() {
                             self.reader_font_size = (self.reader_font_size + 2.0).min(32.0);
                         }

                         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(20.0);

                            // Nav Buttons
                            let current_book = self.get_current_book();
                             if let Some(book) = current_book {
                                let current_idx = book.chapters.iter().position(|c| c.id == chapter_id);
                                if let Some(idx) = current_idx {
                                    // Next
                                    if idx < book.chapters.len() - 1 {
                                        let next = book.chapters[idx + 1].id.clone();
                                        if ui.add(egui::Button::new("Next ▶").min_size(Vec2::new(60.0, 44.0))).clicked() {
                                            // Only save if we're not already loading a new chapter
                                            if self.currently_loading_chapter_id.is_none() {
                                                self.save_chapter_progress(book_id, chapter_id, 1.0);
                                                self.current_view = View::Reader(book_id.to_string(), next.clone());
                                                self.load_chapter(book_id.to_string(), next, ctx);
                                            }
                                        }
                                    }

                                    ui.add_space(12.0);

                                    // Prev
                                    if idx > 0 {
                                        let prev = book.chapters[idx - 1].id.clone();
                                        if ui.add(egui::Button::new("◀ Prev").min_size(Vec2::new(60.0, 44.0))).clicked() {
                                            // Only save if we're not already loading a new chapter
                                            if self.currently_loading_chapter_id.is_none() {
                                                self.save_chapter_progress(book_id, chapter_id, current_progress);
                                                self.current_view = View::Reader(book_id.to_string(), prev.clone());
                                                self.load_chapter(book_id.to_string(), prev, ctx);
                                            }
                                        }
                                    }
                                }
                             }
                         });
                     });
                 });
             });
        }
    }
}

impl App for BookApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Check if book details streaming just completed and save the book
        {
            let mut s = self.state.write().unwrap();
            if s.needs_save_after_streaming {
                if let Some(book) = &s.current_book {
                    if !book.chapters.is_empty() && book.chapters_count > 0 {
                        let book_to_save = book.clone();
                        s.needs_save_after_streaming = false;
                        drop(s); // Release the lock before sending command

                        // Send command to save the book
                        let _ = self.cmd_tx.send(Command::SaveBookToCache { book: Box::new(book_to_save) });
                    }
                }
            }
        }

        // Apply theme
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // If there are pending cover downloads, check periodically for completion
        // This ensures UI updates when covers finish downloading
        if !self.pending_cover_downloads.is_empty() {
            // Remove completed downloads from tracking
            self.pending_cover_downloads.retain(|cache_key| {
                let parts: Vec<&str> = cache_key.splitn(2, '_').collect();
                if parts.len() == 2 {
                    Self::get_cached_cover_url(parts[0], parts[1]).is_none()
                } else {
                    false
                }
            });
            // Request repaint at reduced frequency (500ms) to update UI with newly cached images
            // This prevents excessive redraws while still showing images as they load
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // Render bottom navigation (mobile style) - hide during reading
        if !matches!(self.current_view, View::Reader(_, _)) {
            self.render_bottom_nav(ctx);
        }

        // Render main content
        CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .inner_margin(12.0) // Smaller margin for mobile
                    .fill(if self.dark_mode {
                        Color32::from_rgb(18, 18, 24)
                    } else {
                        Color32::from_rgb(250, 250, 252)
                    }),
            )
            .show(ctx, |ui| match self.current_view.clone() {
                View::Library => self.render_library(ctx, ui),
                View::Discover => self.render_discover(ctx, ui),
                View::Search => self.render_search_page(ctx, ui),
                View::Settings => self.render_settings(ctx, ui),
                View::BookDetails(book_id) => self.render_book_details(ctx, ui, &book_id),
                View::Reader(book_id, chapter_id) => {
                    self.render_reader(ui, &book_id, &chapter_id, ctx)
                }
            });
    }
}

/// Convert HTML content to plain text for reader display
pub fn html_to_plain_text(html: &str) -> String {
    html.replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<p>", "\n")
        .replace("</p>", "\n")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .split('<')
        .map(|s| s.split('>').last().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}
