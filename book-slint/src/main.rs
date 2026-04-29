// ============================================================================
// BOOK APP - Mobile Book Reader (Slint UI)
// ============================================================================

slint::include_modules!();

mod app;

use app::BookApp;

fn main() -> Result<(), slint::PlatformError> {
    // Initialize platform-specific logging
    book_core::platform::init_logging();

    // Create and run the app
    let app = BookApp::new()?;
    app.run()
}
