// Platform abstraction layer for handling platform-specific behaviors
// This module provides a unified interface for platform differences

use std::path::PathBuf;

/// Get the application data directory appropriate for the current platform
pub fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // On Android, use the app's cache directory
        if let Ok(path) = std::env::var("EXTERNAL_CACHE_DIR") {
            return PathBuf::from(path);
        }
        // Fallback to temp
        std::env::temp_dir()
    }

    #[cfg(target_os = "ios")]
    {
        // On iOS, use the Documents directory in the app's sandbox
        use std::path::Path;
        if let Some(home) = dirs::home_dir() {
            return home.join("Documents");
        }
        std::env::temp_dir()
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        // Desktop platforms
        if let Some(data_dir) = dirs::data_dir() {
            return data_dir.join("book-app");
        }
        std::env::temp_dir()
    }
}

/// Get the database path appropriate for the current platform
pub fn get_db_path() -> PathBuf {
    let app_dir = get_app_data_dir();

    #[cfg(target_os = "android")]
    {
        app_dir.join("books.db")
    }

    #[cfg(target_os = "ios")]
    {
        app_dir.join("books.db")
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        // Create directory if it doesn't exist on desktop
        let _ = std::fs::create_dir_all(&app_dir);
        app_dir.join("books.db")
    }
}

/// Get the covers cache directory
pub fn get_covers_dir() -> PathBuf {
    let app_dir = get_app_data_dir();
    let covers_dir = app_dir.join("covers");

    #[cfg(not(target_os = "android"))]
    {
        let _ = std::fs::create_dir_all(&covers_dir);
    }

    covers_dir
}

/// Initialize platform-specific logging
pub fn init_logging() {
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_min_level(log::Level::Debug),
        );
    }

    #[cfg(target_os = "ios")]
    {
        // iOS logging: use standard println! which routes to system log
        // Or use log crate with a simple implementation
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        // Desktop: use standard logging if needed
    }
}

/// Check if running on a mobile platform
pub fn is_mobile() -> bool {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        true
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        false
    }
}

/// Get the optimal UI scale for the current platform
pub fn get_ui_scale() -> f32 {
    #[cfg(target_os = "android")]
    {
        1.0 // Will be scaled by system DPI
    }

    #[cfg(target_os = "ios")]
    {
        1.0 // Will be scaled by system DPI
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        1.0
    }
}
