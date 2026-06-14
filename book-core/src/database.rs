use crate::models::{Book, Chapter, Repository, Source, SourceConfig, SourceWithConfig};
use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let connection = Connection::open("library.db")?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;

        // 1. Repositories Tracking Table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                last_synced_commit TEXT,
                last_checked_timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // 2. Sources Table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                repo_id TEXT,
                url TEXT NOT NULL,
                icon_url TEXT,
                cover_url_pattern TEXT,
                name TEXT NOT NULL,
                description TEXT,
                config TEXT,
                FOREIGN KEY (repo_id) REFERENCES repositories (id) ON DELETE SET NULL
            )",
            [],
        )?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS books (
                id TEXT,
                source_id TEXT,
                in_library BOOLEAN NOT NULL DEFAULT 0,
                title TEXT,
                author TEXT,
                cover_url TEXT,
                rating REAL,
                status TEXT,
                chapters_count INTEGER,
                genres TEXT,
                summary TEXT,
                last_synced INTEGER,
                last_read_timestamp INTEGER DEFAULT 0,
                PRIMARY KEY (source_id, id),
                FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 4. Chapters Table (NEW: Normalized)
        connection.execute(
            "CREATE TABLE IF NOT EXISTS chapters (
                id TEXT,
                book_id TEXT,
                source_id TEXT,
                title TEXT,
                date INTEGER,
                progress REAL DEFAULT 0.0,
                last_read INTEGER DEFAULT 0,
                PRIMARY KEY (source_id, book_id, id),
                FOREIGN KEY (source_id, book_id) REFERENCES books (source_id, id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 5. Chapter Content Cache
        connection.execute(
            "CREATE TABLE IF NOT EXISTS chapter_content (
                book_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                chapter_id TEXT NOT NULL,
                content TEXT NOT NULL,
                cached_at INTEGER NOT NULL,
                PRIMARY KEY (source_id, book_id, chapter_id),
                FOREIGN KEY (source_id, book_id) REFERENCES books (source_id, id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 6. Cover Cache Table
        connection.execute(
            "CREATE TABLE IF NOT EXISTS covers (
                book_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                image_data BLOB NOT NULL,
                cached_at INTEGER NOT NULL,
                PRIMARY KEY (source_id, book_id),
                FOREIGN KEY (source_id, book_id) REFERENCES books (source_id, id) ON DELETE CASCADE
            )",
            [],
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_books_library ON books(in_library)",
            [],
        )?;
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_sources_repo ON sources(repo_id)",
            [],
        )?;
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_chapters_book ON chapters(source_id, book_id)",
            [],
        )?;

        let db = Database { connection };
        Ok(db)
    }

    // ==================== Repositories ====================
    pub fn save_repository(&self, repo: &Repository) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO repositories (id, url, display_name, last_synced_commit, last_checked_timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo.id, repo.url, repo.display_name, repo.last_synced_commit, repo.last_checked_timestamp],
        )?;
        Ok(())
    }

    pub fn get_repository(&self, id: &str) -> Result<Option<Repository>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, url, display_name, last_synced_commit, last_checked_timestamp FROM repositories WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map([id], |row| {
            Ok(Repository {
                id: row.get(0)?,
                url: row.get(1)?,
                display_name: row.get(2)?,
                last_synced_commit: row.get(3)?,
                last_checked_timestamp: row.get(4)?,
            })
        })?;
        if let Some(res) = rows.next() {
            return Ok(Some(res?));
        }
        Ok(None)
    }

    // ==================== Books ====================
    pub fn save_book(&self, book: &Book) -> Result<()> {
        // Use transaction for atomic save
        let tx = self.connection.unchecked_transaction()?;
        let genres_json = serde_json::to_string(&book.genres).unwrap_or_default();
        let timestamp = Utc::now().timestamp();

        tx.execute(
        "INSERT OR REPLACE INTO books (id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, last_synced, last_read_timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            book.id, book.source_id, book.in_library, book.title, book.author, book.cover_url,
            book.rating, book.status, book.chapters_count, genres_json, book.summary, timestamp, book.last_read_timestamp
        ],
    )?;

        // Clear old chapters to prevent orphaned data if chapters were removed from source
        tx.execute(
            "DELETE FROM chapters WHERE source_id = ?1 AND book_id = ?2",
            params![book.source_id, book.id],
        )?;

        {
            let mut insert_chapter_stmt = tx.prepare(
                "INSERT INTO chapters (id, book_id, source_id, title, date, progress, last_read)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;

            for chapter in &book.chapters {
                insert_chapter_stmt.execute(params![
                    chapter.id,
                    book.id,
                    book.source_id,
                    chapter.title,
                    chapter.date,
                    chapter.progress,
                    chapter.last_read
                ])?;
            }
        } // insert_chapter_stmt is dropped here, releasing the borrow on tx

        tx.commit()?;
        Ok(())
    }

    fn build_book_from_row(&self, row: &rusqlite::Row) -> rusqlite::Result<Book> {
        let genres_json: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
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
            genres: serde_json::from_str(&genres_json).unwrap_or_default(),
            summary: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            last_read_timestamp: row.get::<_, Option<i64>>(12)?.unwrap_or(0),
            chapters: Vec::new(), // Populated separately
        })
    }

    fn fetch_chapters_for_book(&self, book_id: &str, source_id: &str) -> Result<Vec<Chapter>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, date, progress, last_read FROM chapters WHERE source_id = ?1 AND book_id = ?2 ORDER BY date DESC, id DESC"
        )?;
        let rows = stmt.query_map(params![source_id, book_id], |row| {
            Ok(Chapter {
                id: row.get(0)?,
                title: row.get(1)?,
                date: row.get(2)?,
                progress: row.get(3)?,
                last_read: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_book(&self, id: &str, source_id: &str) -> Result<Option<Book>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, last_synced, last_read_timestamp
            FROM books WHERE id = ?1 AND source_id = ?2"
        )?;
        let mut rows =
            stmt.query_map(params![id, source_id], |row| self.build_book_from_row(row))?;

        let mut book = match rows.next() {
            Some(res) => res?,
            None => return Ok(None),
        };

        book.chapters = self.fetch_chapters_for_book(id, source_id)?;
        Ok(Some(book))
    }

    pub fn get_library_books(&self) -> Result<Vec<Book>> {
        let mut books = Vec::new();
        let mut stmt = self.connection.prepare(
            "SELECT id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, last_synced, last_read_timestamp
            FROM books WHERE in_library = 1 ORDER BY last_read_timestamp DESC"
        )?;

        let book_rows = stmt.query_map([], |row| self.build_book_from_row(row))?;
        for book_res in book_rows {
            let mut book = book_res?;
            book.chapters = self.fetch_chapters_for_book(&book.id, &book.source_id)?;
            books.push(book);
        }
        Ok(books)
    }

    pub fn get_all_books(&self) -> Result<Vec<Book>> {
        let mut books = Vec::new();
        let mut stmt = self.connection.prepare(
            "SELECT id, source_id, in_library, title, author, cover_url, rating, status, chapters_count, genres, summary, last_synced, last_read_timestamp FROM books"
        )?;

        let book_rows = stmt.query_map([], |row| self.build_book_from_row(row))?;
        for book_res in book_rows {
            let mut book = book_res?;
            book.chapters = self.fetch_chapters_for_book(&book.id, &book.source_id)?;
            books.push(book);
        }
        Ok(books)
    }

    pub fn remove_from_library(&self, book_id: &str, source_id: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE books SET in_library = 0 WHERE id = ?1 AND source_id = ?2",
            params![book_id, source_id],
        )?;
        Ok(())
    }

    pub fn delete_book(&self, id: &str, source_id: &str) -> Result<usize> {
        self.connection.execute(
            "DELETE FROM books WHERE id = ?1 AND source_id = ?2",
            params![id, source_id],
        )
    }

    // FIX: Atomic update prevents Read-Modify-Write race conditions
    pub fn update_chapter_progress(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
        progress: f32,
    ) -> Result<()> {
        let timestamp = Utc::now().timestamp();
        self.connection.execute(
            "UPDATE chapters SET progress = ?1, last_read = ?2 WHERE source_id = ?3 AND book_id = ?4 AND id = ?5",
            params![progress, timestamp, source_id, book_id, chapter_id],
        )?;

        // Optionally update the book's last_read_timestamp if the chapter is fully read
        if progress >= 1.0 {
            self.connection.execute(
                "UPDATE books SET last_read_timestamp = ?1 WHERE id = ?2 AND source_id = ?3",
                params![timestamp, book_id, source_id],
            )?;
        }
        Ok(())
    }

    pub fn mark_chapter_read(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
    ) -> Result<()> {
        self.update_chapter_progress(book_id, source_id, chapter_id, 1.0)
    }

    // ==================== Sources ====================
    pub fn save_source_with_repo(
        &self,
        source: &SourceWithConfig,
        repo_id: Option<&str>,
    ) -> Result<()> {
        let config_json = serde_json::to_string(&source.config).unwrap_or_default();
        self.connection.execute(
            "INSERT OR REPLACE INTO sources (id, repo_id, url, cover_url_pattern, name, icon_url, description, config)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                source.source.id, repo_id, source.source.url, source.source.cover_url_pattern,
                source.source.name, source.source.icon_url, source.source.description, config_json
            ],
        )?;
        Ok(())
    }

    pub fn get_source(&self, id: &str) -> Result<Option<SourceWithConfig>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, url, cover_url_pattern, name, icon_url, description, config FROM sources WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map([id], |row| {
            let config_json: String = row.get(6)?;
            let config = serde_json::from_str(&config_json).unwrap_or_default();
            Ok(SourceWithConfig {
                source: Source {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    cover_url_pattern: row.get(2)?,
                    name: row.get(3)?,
                    icon_url: row.get(4)?,
                    description: row.get(5)?,
                },
                config,
            })
        })?;
        if let Some(r) = rows.next() {
            return Ok(Some(r?));
        }
        Ok(None)
    }
        /// Returns all sources along with their full scraping/parsing configuration.
    pub fn get_sources(&self) -> Result<Vec<SourceWithConfig>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, url, cover_url_pattern, name, icon_url, description, config FROM sources"
        )?;

        let source_iter = stmt.query_map([], |row| {
            let config_json: String = row.get(6)?;
            let config = serde_json::from_str(&config_json).unwrap_or_default();

            Ok(SourceWithConfig {
                source: Source {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    cover_url_pattern: row.get(2)?,
                    name: row.get(3)?,
                    icon_url: row.get(4)?,
                    description: row.get(5)?,
                },
                config,
            })
        })?;

        source_iter.collect()
    }

    // ==================== Chapter Content Cache ====================
    pub fn cache_chapter_content(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
        content: &str,
    ) -> Result<()> {
        let cached_at = Utc::now().timestamp();
        self.connection.execute(
            "INSERT OR REPLACE INTO chapter_content (book_id, source_id, chapter_id, content, cached_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![book_id, source_id, chapter_id, content, cached_at],
        )?;
        Ok(())
    }

    pub fn get_cached_chapter_content(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
    ) -> Result<Option<String>> {
        let mut stmt = self.connection.prepare("SELECT content FROM chapter_content WHERE book_id = ?1 AND source_id = ?2 AND chapter_id = ?3")?;
        let mut rows = stmt.query_map(params![book_id, source_id, chapter_id], |row| {
            row.get::<_, String>(0)
        })?;
        if let Some(result) = rows.next() {
            return Ok(Some(result?));
        }
        Ok(None)
    }

    // ==================== Cover Cache ====================
    pub fn cache_cover(&self, book_id: &str, source_id: &str, image_data: &[u8]) -> Result<()> {
        let cached_at = Utc::now().timestamp();
        self.connection.execute(
            "INSERT OR REPLACE INTO covers (book_id, source_id, image_data, cached_at) VALUES (?1, ?2, ?3, ?4)",
            params![book_id, source_id, image_data, cached_at],
        )?;
        Ok(())
    }

    pub fn get_cached_cover(&self, book_id: &str, source_id: &str) -> Result<Option<Vec<u8>>> {
        let mut stmt = self
            .connection
            .prepare("SELECT image_data FROM covers WHERE book_id = ?1 AND source_id = ?2")?;
        let mut rows =
            stmt.query_map(params![book_id, source_id], |row| row.get::<_, Vec<u8>>(0))?;
        if let Some(result) = rows.next() {
            return Ok(Some(result?));
        }
        Ok(None)
    }

    pub fn close(self) -> Result<()> {
        self.connection.close().map_err(|(_, err)| err)
    }
}
