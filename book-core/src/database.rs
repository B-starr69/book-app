use crate::models::{Book, CacheStats, Chapter, DbBook, DbChapter, Source, SourceConfig, SourceWithConfig};
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let connection = Connection::open("library.db")?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;

        // 1. Sources Table (with config JSON column)
        connection.execute(
            "CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                name TEXT NOT NULL,
                discover_url TEXT,
                books_url TEXT,
                icon_url TEXT,
                description TEXT,
                origin_repo TEXT,
                origin_commit TEXT,
                config TEXT
                )",
            [],
        )?;

        // 2. Books Table (with metadata)
        connection.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id TEXT,
                source_id TEXT,
                in_library BOOLEAN NOT NULL,
                title TEXT,
                author TEXT,
                cover_url TEXT,
                rating REAL,
                status TEXT,
                chapters_count INTEGER,
                genres TEXT,
                summary TEXT,
                chapters_json TEXT,
                PRIMARY KEY (source_id, id),
                FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE
            )",
            [],
        )?;


        // 3. Chapters Table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS chapters (
                id TEXT,
                book_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                progress REAL NOT NULL DEFAULT 0,
                last_read INTEGER,
                PRIMARY KEY (source_id, id),
                FOREIGN KEY (book_id, source_id) REFERENCES books (id, source_id) ON DELETE CASCADE
            )",
            [],
        )?;



        connection.execute(
            "CREATE TABLE IF NOT EXISTS chapter_content (
                book_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                chapter_id TEXT NOT NULL,
                content TEXT NOT NULL,
                cached_at INTEGER NOT NULL,
                PRIMARY KEY (source_id, book_id, chapter_id)
            )",
            [],
        )?;

        // 5. Cover Cache Table (no FK - cache works for any book)
        connection.execute(
            "CREATE TABLE IF NOT EXISTS covers (
                book_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                image_data BLOB NOT NULL,
                cached_at INTEGER NOT NULL,
                PRIMARY KEY (source_id, book_id)
            )",
            [],
        )?;

        // Migration: Add last_synced column to books if it doesn't exist
        let _ = connection.execute("ALTER TABLE books ADD COLUMN last_synced INTEGER", []);

        Ok(Database { connection })
    }

    // ==================== Books ====================

    /// Insert a book with minimal info (for backward compatibility)
    pub fn save_book(&self, book: &DbBook) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO books (id, source_id, in_library)
             VALUES (?1, ?2, ?3)",
            params![book.id, book.source_id, book.in_library],
        )?;
        Ok(())
    }

    /// Save a full book with all metadata to library
    pub fn save_full_book(&self, book: &Book) -> Result<()> {
        let genres_json = serde_json::to_string(&book.genres).unwrap_or_default();
        let chapters_json = serde_json::to_string(&book.chapters).unwrap_or_default();
        let timestamp = Utc::now().timestamp();

        self.connection.execute(
            "INSERT OR REPLACE INTO books (id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, chapters_json, last_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                book.id,
                book.source_id,
                book.in_library,
                book.title,
                book.author,
                book.cover_url,
                book.rating,
                book.status,
                book.chapters_count,
                genres_json,
                book.summary,
                chapters_json,
                timestamp
            ],
        )?;
        Ok(())
    }

    /// Get all books in library with full metadata
    pub fn get_library_books(&self) -> Result<Vec<Book>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, chapters_json
             FROM books WHERE in_library = 1"
        )?;

        let rows = stmt.query_map([], |row| {
            let genres_json: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
            let chapters_json: String = row.get::<_, Option<String>>(11)?.unwrap_or_default();

            let genres: Vec<String> = serde_json::from_str(&genres_json).unwrap_or_default();
            let chapters: Vec<Chapter> = serde_json::from_str(&chapters_json).unwrap_or_default();

            Ok(Book {
                id: row.get(0)?,
                source_id: row.get(1)?,
                in_library: row.get(2)?,
                title: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                author: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                cover_url: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                rating: row.get::<_, Option<f32>>(6)?.unwrap_or(0.0),
                status: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                chapters_count: row.get::<_, Option<i32>>(8)?.unwrap_or(0),
                genres,
                summary: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                chapters,
            })
        })?;

        rows.collect()
    }

    /// Remove a book from library
    pub fn remove_from_library(&self, book_id: &str, source_id: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE books SET in_library = 0 WHERE id = ?1 AND source_id = ?2",
            params![book_id, source_id],
        )?;
        Ok(())
    }

    /// Get a full book by id (whether in library or just cached)
    pub fn get_full_book(&self, id: &str, source_id: &str) -> Result<Option<Book>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, chapters_json
             FROM books WHERE id = ?1 AND source_id = ?2",
        )?;

        let mut rows = stmt.query_map(params![id, source_id], |row| {
            let genres_json: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
            let chapters_json: String = row.get::<_, Option<String>>(11)?.unwrap_or_default();

            let genres: Vec<String> = serde_json::from_str(&genres_json).unwrap_or_default();
            let chapters: Vec<Chapter> = serde_json::from_str(&chapters_json).unwrap_or_default();

            Ok(Book {
                id: row.get(0)?,
                source_id: row.get(1)?,
                in_library: row.get(2)?,
                title: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                author: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                cover_url: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                rating: row.get::<_, Option<f32>>(6)?.unwrap_or(0.0),
                status: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                chapters_count: row.get::<_, Option<i32>>(8)?.unwrap_or(0),
                genres,
                summary: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                chapters,
            })
        })?;

        if let Some(result) = rows.next() {
            return Ok(Some(result?));
        }
        Ok(None)
    }

    pub fn get_db_book(&self, id: &str, source_id: &str) -> Result<Option<DbBook>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, source_id, in_library FROM books WHERE id = ?1 AND source_id = ?2",
        )?;

        let mut rows = stmt.query_map(params![id, source_id], |row| {
            Ok(DbBook {
                id: row.get(0)?,
                source_id: row.get(1)?,
                in_library: row.get(2)?,
            })
        })?;

        if let Some(res) = rows.next() {
            return Ok(Some(res?));
        }
        Ok(None)
    }

    pub fn get_all_db_books(&self) -> Result<Vec<DbBook>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, source_id, in_library FROM books")?;

        let rows = stmt.query_map(params![], |row| {
            Ok(DbBook {
                id: row.get(0)?,
                source_id: row.get(1)?,
                in_library: row.get(2)?,
            })
        })?;

        rows.collect()
    }

    /// Delete a book and its chapters (cascade)
    pub fn delete_book(&self, id: &str, source_id: &str) -> Result<usize> {
        self.connection.execute(
            "DELETE FROM books WHERE id = ?1 AND source_id = ?2",
            params![id, source_id],
        )
    }

    // ==================== Chapters ====================

    /// Upsert progress for chapters
    pub fn save_chapters_progress(&mut self, chapters: &[DbChapter]) -> Result<()> {
        let tx = self.connection.transaction()?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO chapters (id, book_id, source_id, progress, last_read)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(source_id, id)
                 DO UPDATE SET progress = excluded.progress, last_read = excluded.last_read",
            )?;

            for chap in chapters {
                stmt.execute(params![
                    chap.id,
                    chap.book_id,
                    chap.source_id,
                    chap.progress,
                    chap.last_read
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_chapters_for_book(&self, book_id: &str, source_id: &str) -> Result<Vec<DbChapter>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, book_id, source_id, progress, last_read
             FROM chapters
             WHERE book_id = ?1 AND source_id = ?2
             ORDER BY id ASC",
        )?;

        let chapter_iter = stmt.query_map(params![book_id, source_id], |row| {
            Ok(DbChapter {
                id: row.get(0)?,
                book_id: row.get(1)?,
                source_id: row.get(2)?,
                progress: row.get(3)?,
                last_read: row.get(4)?,
            })
        })?;

        chapter_iter.collect()
    }

    pub fn get_chapters(&self) -> Result<Vec<DbChapter>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, book_id, source_id, progress, last_read
             FROM chapters",
        )?;

        let chapter_iter = stmt.query_map([], |row| {
            Ok(DbChapter {
                id: row.get(0)?,
                book_id: row.get(1)?,
                source_id: row.get(2)?,
                progress: row.get(3)?,
                last_read: row.get(4)?,
            })
        })?;

        chapter_iter.collect()
    }

    /// Update chapter progress (legacy - may not work with composite key)
    pub fn update_chapter_progress(&self, chapter_id: &str, progress: f32) -> Result<()> {
        let timestamp_seconds = Utc::now().timestamp();
        self.connection.execute(
            "UPDATE chapters SET progress = ?1, last_read = ?2 WHERE id = ?3",
            params![progress, timestamp_seconds, chapter_id],
        )?;
        Ok(())
    }

    /// Mark a chapter as read (upsert with proper composite key)
    pub fn mark_chapter_read(&self, chapter_id: &str, book_id: &str, source_id: &str) -> Result<()> {
        let timestamp_seconds = Utc::now().timestamp();
        self.connection.execute(
            "INSERT INTO chapters (id, book_id, source_id, progress, last_read)
             VALUES (?1, ?2, ?3, 1.0, ?4)
             ON CONFLICT(source_id, id) DO UPDATE SET progress = 1.0, last_read = excluded.last_read",
            params![chapter_id, book_id, source_id, timestamp_seconds],
        )?;
        Ok(())
    }

    // ==================== Sources ====================

    /// Insert or update a source (without config, for backward compatibility)
    pub fn save_source(&self, source: &Source) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO sources (id, url, name, discover_url, books_url, icon_url, description, config)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, (SELECT config FROM sources WHERE id = ?1))",
            params![
                source.id,
                source.url,
                source.name,
                source.discover_url,
                source.books_url,
                source.icon_url,
                source.description
            ],
        )?;
        Ok(())
    }

    /// Insert or update a source with its configuration
    pub fn save_source_with_config(&self, source: &SourceWithConfig) -> Result<()> {
        let config_json = serde_json::to_string(&source.config).unwrap_or_default();
        self.connection.execute(
            "INSERT OR REPLACE INTO sources (id, url, name, discover_url, books_url, icon_url, description, config)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                source.id,
                source.url,
                source.name,
                source.discover_url,
                source.books_url,
                source.icon_url,
                source.description,
                config_json
            ],
        )?;
        Ok(())
    }

    /// Update only the config for a source
    pub fn update_source_config(&self, source_id: &str, config: &SourceConfig) -> Result<()> {
        let config_json = serde_json::to_string(config).unwrap_or_default();
        self.connection.execute(
            "UPDATE sources SET config = ?1 WHERE id = ?2",
            params![config_json, source_id],
        )?;
        Ok(())
    }

    /// Update the origin repository and commit SHA for a source
    pub fn update_source_origin(&self, source_id: &str, origin_repo: &str, origin_commit: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE sources SET origin_repo = ?1, origin_commit = ?2 WHERE id = ?3",
            params![origin_repo, origin_commit, source_id],
        )?;
        Ok(())
    }

    /// Delete a source (cascades to books and chapters)
    pub fn delete_source(&self, id: &str) -> Result<usize> {
        self.connection
            .execute("DELETE FROM sources WHERE id = ?1", params![id])
    }

    /// Return a source by id (without config)
    pub fn get_source(&self, id: &str) -> Result<Option<Source>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, url, name, discover_url, books_url, icon_url, description, origin_repo, origin_commit FROM sources WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Source {
                id: row.get(0)?,
                url: row.get(1)?,
                name: row.get(2)?,
                discover_url: row.get(3).unwrap_or_default(),
                books_url: row.get(4).unwrap_or_default(),
                icon_url: row.get::<_, Option<String>>(5)?,
                description: row.get::<_, Option<String>>(6)?,
            })
        })?;
        if let Some(r) = rows.next() {
            return Ok(Some(r?));
        }
        Ok(None)
    }

    /// Return a source with its config by id
    pub fn get_source_with_config(&self, id: &str) -> Result<Option<SourceWithConfig>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, url, name, discover_url, books_url, icon_url, description, origin_repo, origin_commit, config FROM sources WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |row| {
            let config_json: Option<String> = row.get(9).ok();
            let config: SourceConfig = config_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            Ok(SourceWithConfig {
                id: row.get(0)?,
                url: row.get(1)?,
                name: row.get(2)?,
                discover_url: row.get(3).unwrap_or_default(),
                books_url: row.get(4).unwrap_or_default(),
                icon_url: row.get::<_, Option<String>>(5)?,
                description: row.get::<_, Option<String>>(6)?,
                config,
            })
        })?;
        if let Some(r) = rows.next() {
            return Ok(Some(r?));
        }
        Ok(None)
    }

    /// Get all sources (without configs, for listing)
    pub fn get_sources(&self) -> Result<Vec<Source>> {
        let sql: &str = "SELECT id, url, name, discover_url, books_url, icon_url, description, origin_repo, origin_commit FROM sources";
        let mut stmt = self.connection.prepare(sql)?;

        let source_iter = stmt.query_map([], |row| {
            Ok(Source {
                id: row.get(0)?,
                url: row.get(1)?,
                name: row.get(2)?,
                discover_url: row.get(3).unwrap_or_default(),
                books_url: row.get(4).unwrap_or_default(),
                icon_url: row.get::<_, Option<String>>(5)?,
                description: row.get::<_, Option<String>>(6)?,
            })
        })?;

        source_iter.collect()
    }

    /// Get all sources with their configs
    pub fn get_sources_with_config(&self) -> Result<Vec<SourceWithConfig>> {
        let sql = "SELECT id, url, name, discover_url, books_url, icon_url, description, origin_repo, origin_commit, config FROM sources";
        let mut stmt = self.connection.prepare(sql)?;

        let source_iter = stmt.query_map([], |row| {
            let config_json: Option<String> = row.get(7).ok();
            let config: SourceConfig = config_json
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();
            Ok(SourceWithConfig {
                id: row.get(0)?,
                url: row.get(1)?,
                name: row.get(2)?,
                discover_url: row.get(3).unwrap_or_default(),
                books_url: row.get(4).unwrap_or_default(),
                icon_url: row.get::<_, Option<String>>(5)?,
                description: row.get::<_, Option<String>>(6)?,
                config,
            })
        })?;

        source_iter.collect()
    }

    /// Get sources that were imported from a specific origin repository
    pub fn get_sources_by_origin(&self, origin_repo: &str) -> Result<Vec<(String, Option<String>)>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, origin_commit FROM sources WHERE origin_repo = ?1")?;

        let rows = stmt.query_map(params![origin_repo], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;

        rows.collect()
    }

    // ==================== Chapter Content Cache ====================

    /// Save chapter content to cache
    pub fn cache_chapter_content(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
        content: &str,
    ) -> Result<()> {
        let cached_at = Utc::now().timestamp();
        self.connection.execute(
            "INSERT OR REPLACE INTO chapter_content (book_id, source_id, chapter_id, content, cached_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![book_id, source_id, chapter_id, content, cached_at],
        )?;
        Ok(())
    }

    /// Get cached chapter content
    pub fn get_cached_chapter_content(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
    ) -> Result<Option<String>> {
        let mut stmt = self.connection.prepare(
            "SELECT content FROM chapter_content
             WHERE book_id = ?1 AND source_id = ?2 AND chapter_id = ?3",
        )?;

        let mut rows = stmt.query_map(params![book_id, source_id, chapter_id], |row| {
            row.get::<_, String>(0)
        })?;

        if let Some(result) = rows.next() {
            return Ok(Some(result?));
        }
        Ok(None)
    }

    /// Check if chapter content is cached
    pub fn is_chapter_cached(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
    ) -> Result<bool> {
        let mut stmt = self.connection.prepare(
            "SELECT 1 FROM chapter_content
             WHERE book_id = ?1 AND source_id = ?2 AND chapter_id = ?3",
        )?;

        let exists = stmt.exists(params![book_id, source_id, chapter_id])?;
        Ok(exists)
    }

    /// Delete cached chapter content for a book
    pub fn clear_chapter_cache(&self, book_id: &str, source_id: &str) -> Result<usize> {
        self.connection.execute(
            "DELETE FROM chapter_content WHERE book_id = ?1 AND source_id = ?2",
            params![book_id, source_id],
        )
    }

    /// Get count of cached chapters for a book
    pub fn get_cached_chapter_count(&self, book_id: &str, source_id: &str) -> Result<i32> {
        let mut stmt = self.connection.prepare(
            "SELECT COUNT(*) FROM chapter_content WHERE book_id = ?1 AND source_id = ?2",
        )?;

        let count: i32 = stmt.query_row(params![book_id, source_id], |row| row.get(0))?;
        Ok(count)
    }

    // ==================== Cover Cache ====================

    /// Save cover image to cache
    pub fn cache_cover(&self, book_id: &str, source_id: &str, image_data: &[u8]) -> Result<()> {
        let cached_at = Utc::now().timestamp();
        self.connection.execute(
            "INSERT OR REPLACE INTO covers (book_id, source_id, image_data, cached_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![book_id, source_id, image_data, cached_at],
        )?;
        Ok(())
    }

    /// Get cached cover image
    pub fn get_cached_cover(&self, book_id: &str, source_id: &str) -> Result<Option<Vec<u8>>> {
        let mut stmt = self.connection.prepare(
            "SELECT image_data FROM covers WHERE book_id = ?1 AND source_id = ?2",
        )?;

        let mut rows = stmt.query_map(params![book_id, source_id], |row| {
            row.get::<_, Vec<u8>>(0)
        })?;

        if let Some(result) = rows.next() {
            return Ok(Some(result?));
        }
        Ok(None)
    }

    /// Check if cover is cached
    pub fn is_cover_cached(&self, book_id: &str, source_id: &str) -> Result<bool> {
        let mut stmt = self.connection.prepare(
            "SELECT 1 FROM covers WHERE book_id = ?1 AND source_id = ?2",
        )?;

        let exists = stmt.exists(params![book_id, source_id])?;
        Ok(exists)
    }

    /// Delete cached cover for a book
    pub fn delete_cached_cover(&self, book_id: &str, source_id: &str) -> Result<usize> {
        self.connection.execute(
            "DELETE FROM covers WHERE book_id = ?1 AND source_id = ?2",
            params![book_id, source_id],
        )
    }

    // ==================== Sync Tracking ====================

    /// Update last_synced timestamp for a book
    pub fn update_last_synced(&self, book_id: &str, source_id: &str) -> Result<()> {
        let timestamp = Utc::now().timestamp();
        self.connection.execute(
            "UPDATE books SET last_synced = ?1 WHERE id = ?2 AND source_id = ?3",
            params![timestamp, book_id, source_id],
        )?;
        Ok(())
    }

    /// Get last_synced timestamp for a book
    pub fn get_last_synced(&self, book_id: &str, source_id: &str) -> Result<Option<i64>> {
        let mut stmt = self.connection.prepare(
            "SELECT last_synced FROM books WHERE id = ?1 AND source_id = ?2",
        )?;

        let mut rows = stmt.query_map(params![book_id, source_id], |row| {
            row.get::<_, Option<i64>>(0)
        })?;

        if let Some(result) = rows.next() {
            return Ok(result?);
        }
        Ok(None)
    }

    /// Check if a book needs to be re-synced (older than specified days)
    pub fn needs_sync(&self, book_id: &str, source_id: &str, max_age_days: i64) -> bool {
        match self.get_last_synced(book_id, source_id) {
            Ok(Some(last_synced)) => {
                let now = Utc::now().timestamp();
                let age_seconds = now - last_synced;
                let max_age_seconds = max_age_days * 24 * 60 * 60;
                age_seconds > max_age_seconds
            }
            _ => true, // No last_synced means it needs sync
        }
    }

    // ==================== Cache Statistics ====================

    /// Get total cache size in bytes (approximate)
    pub fn get_cache_stats(&self) -> Result<CacheStats> {
        let chapter_count: i32 = self.connection.query_row(
            "SELECT COUNT(*) FROM chapter_content",
            [],
            |row| row.get(0),
        )?;

        let cover_count: i32 = self.connection.query_row(
            "SELECT COUNT(*) FROM covers",
            [],
            |row| row.get(0),
        )?;

        // Approximate sizes
        let chapter_size: i64 = self.connection.query_row(
            "SELECT COALESCE(SUM(LENGTH(content)), 0) FROM chapter_content",
            [],
            |row| row.get(0),
        )?;

        let cover_size: i64 = self.connection.query_row(
            "SELECT COALESCE(SUM(LENGTH(image_data)), 0) FROM covers",
            [],
            |row| row.get(0),
        )?;

        Ok(CacheStats {
            chapter_count,
            cover_count,
            chapter_size_bytes: chapter_size,
            cover_size_bytes: cover_size,
            total_size_bytes: chapter_size + cover_size,
        })
    }

    /// Clear all cached data (chapters and covers)
    pub fn clear_all_cache(&self) -> Result<()> {
        self.connection.execute("DELETE FROM chapter_content", [])?;
        self.connection.execute("DELETE FROM covers", [])?;
        Ok(())
    }

    pub fn close(self) -> Result<()> {
        self.connection.close().map_err(|(_, err)| err)
    }
}
