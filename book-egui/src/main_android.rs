// ============================================================================
// BOOK APP - Mobile Book Reader (Android)
// ============================================================================
//
// This is the Android entry point for the application.
// The Android NDK will call this as the main application.

#![allow(non_snake_case)]

mod app;
mod cover_cache;
mod logic;
mod state;
mod ui;

use app::BookApp;
use eframe::egui;
use egui::{Color32, Context, Vec2};
use jni::JNIEnv;
use ndk_context::AndroidContext;

// Theme colors - used across modules
pub const ACCENT_COLOR: Color32 = Color32::from_rgb(99, 102, 241); // Indigo
pub const SUCCESS_COLOR: Color32 = Color32::from_rgb(34, 197, 94); // Green
pub const WARNING_COLOR: Color32 = Color32::from_rgb(251, 191, 36); // Amber
pub const CARD_BG_DARK: Color32 = Color32::from_rgb(30, 30, 40);
pub const CARD_BG_LIGHT: Color32 = Color32::from_rgb(245, 245, 250);
pub const SIDEBAR_BG_DARK: Color32 = Color32::from_rgb(20, 20, 28);
pub const SIDEBAR_BG_LIGHT: Color32 = Color32::from_rgb(235, 235, 245);

pub fn main() -> eframe::Result<()> {
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

/// Android JNI entry point
/// This function is called from Java code to start the application
#[no_mangle]
pub extern "C" fn Java_com_bookapp_BookAppActivity_nativeStart(env: JNIEnv, _class: jni::objects::JClass) {
    // Initialize the Android context for the NDK
    if let Ok(ctx) = ndk_context::android_context() {
        // Application startup logic can be placed here
        drop(ctx);
    }

    // Run the main application
    let _ = main();
}

/// Setup custom fonts for the application
pub fn setup_custom_fonts(ctx: &Context) {
    let fonts = egui::FontDefinitions::default();
    ctx.set_fonts(fonts);
}
