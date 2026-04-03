// ============================================================================
// BOOK APP - Mobile Book Reader (iOS)
// ============================================================================
//
// This is the iOS entry point for the application.
// iOS will launch this through the standard Rust main function.

mod app;
mod cover_cache;
mod logic;
mod state;
mod ui;

use app::BookApp;
use eframe::egui;
use egui::{Color32, Context, Vec2};

// Theme colors - used across modules
pub const ACCENT_COLOR: Color32 = Color32::from_rgb(99, 102, 241); // Indigo
pub const SUCCESS_COLOR: Color32 = Color32::from_rgb(34, 197, 94); // Green
pub const WARNING_COLOR: Color32 = Color32::from_rgb(251, 191, 36); // Amber
pub const CARD_BG_DARK: Color32 = Color32::from_rgb(30, 30, 40);
pub const CARD_BG_LIGHT: Color32 = Color32::from_rgb(245, 245, 250);
pub const SIDEBAR_BG_DARK: Color32 = Color32::from_rgb(20, 20, 28);
pub const SIDEBAR_BG_LIGHT: Color32 = Color32::from_rgb(235, 235, 245);

fn main() -> eframe::Result<()> {
    // Initialize platform-specific logging
    book_core::platform::init_logging();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(Vec2::new(390.0, 844.0))
            .with_min_inner_size(Vec2::new(320.0, 480.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Book App",
        options,
        Box::new(|cc| Ok(Box::new(BookApp::new(cc)))),
    )
}

/// Setup custom fonts for the application
pub fn setup_custom_fonts(ctx: &Context) {
    let fonts = egui::FontDefinitions::default();
    ctx.set_fonts(fonts);
}
