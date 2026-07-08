use crate::models::{
    BaseBook, Book, BookFormat, Chapter, Novel, Repository, Source, SourceType, SourceWithConfig,
    WebNovel,
};
use crate::platform;
use crate::storage;
use chrono::Utc;
use libsql::{params, Builder, Connection, Database as LibsqlDatabase};
use std::collections::HashMap;

// =========================================================================
// Connection helpers
// =========================================================================
pub struct TursoConfig {
    pub url: String,
    pub auth_token: String,
}

pub enum DatabaseMode {
    Local {
        path: String,
    },
    Remote(TursoConfig),
    RemoteReplica {
        local_path: String,
        remote: TursoConfig,
    },
}

// =========================================================================
// Database
// =========================================================================
pub struct Database {
    _db: LibsqlDatabase,
    conn: Connection,
}

impl Database {
    pub async fn open_local() -> Result<Self, libsql::Error> {
        let path = platform::get_db_path();
        if let Some(parent) = path.parent() {
            println!("{:?}", parent);
            let _ = std::fs::create_dir_all(parent);
        }
        Self::new(DatabaseMode::Local {
            path: path.to_string_lossy().into_owned(),
        })
        .await
    }

    pub async fn new(mode: DatabaseMode) -> Result<Self, libsql::Error> {
        let db = match mode {
            DatabaseMode::Local { path } => Builder::new_local(path).build().await?,
            DatabaseMode::Remote(cfg) => {
                Builder::new_remote(cfg.url, cfg.auth_token).build().await?
            }
            DatabaseMode::RemoteReplica { local_path, remote } => {
                Builder::new_remote_replica(local_path, remote.url, remote.auth_token)
                    .build()
                    .await?
            }
        };

        let conn = db.connect()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;").await?;

        let instance = Database { _db: db, conn };
        instance.init_schema().await?;
        instance.seed_local_source().await?;
        Ok(instance)
    }

    // ------------------------------------------------------------------
    // Schema
    // ------------------------------------------------------------------
    async fn init_schema(&self) -> Result<(), libsql::Error> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY, url TEXT NOT NULL UNIQUE, display_name TEXT NOT NULL,
                last_synced_commit TEXT, last_checked_timestamp INTEGER NOT NULL);",
            )
            .await?;

        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY, repo_id TEXT, url TEXT NOT NULL, icon_url TEXT,
                cover_url_pattern TEXT, name TEXT NOT NULL, description TEXT, config TEXT,
                default_format TEXT NOT NULL DEFAULT 'web_novel',
                FOREIGN KEY (repo_id) REFERENCES repositories (id) ON DELETE SET NULL);",
            )
            .await?;

        // FIX: Removed `genres TEXT` column to enforce proper normalization
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS books (
                id TEXT, source_id TEXT NOT NULL,
                in_library BOOLEAN NOT NULL DEFAULT 0, title TEXT, author TEXT, cover_url TEXT,
                rating REAL, status TEXT, summary TEXT, last_synced INTEGER,
                last_read_timestamp INTEGER DEFAULT 0,
                PRIMARY KEY (source_id, id),
                FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE);",
            )
            .await?;

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS novels (
                book_id TEXT NOT NULL, source_id TEXT NOT NULL, format TEXT NOT NULL,
                file_path TEXT,
                progress REAL NOT NULL DEFAULT 0.0,
                PRIMARY KEY (source_id, book_id),
                FOREIGN KEY (source_id, book_id) REFERENCES books (source_id, id) ON DELETE CASCADE);",
        ).await?;

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS webnovels (
                book_id TEXT NOT NULL, source_id TEXT NOT NULL, chapters_count INTEGER NOT NULL DEFAULT 0,
                chapters_path TEXT NOT NULL,
                PRIMARY KEY (source_id, book_id),
                FOREIGN KEY (source_id, book_id) REFERENCES books (source_id, id) ON DELETE CASCADE);",
        ).await?;

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS chapters (
                id TEXT NOT NULL, book_id TEXT NOT NULL, source_id TEXT NOT NULL, title TEXT,
                file_path TEXT, date INTEGER, progress REAL DEFAULT 0.0, last_read INTEGER DEFAULT 0,
                PRIMARY KEY (source_id, book_id, id),
                FOREIGN KEY (source_id, book_id) REFERENCES webnovels (source_id, book_id) ON DELETE CASCADE);",
        ).await?;

        // FIX: Restored proper normalized tables for Genres
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS genres (
                id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE);",
            )
            .await?;

        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS book_genres (
                book_id TEXT NOT NULL, source_id TEXT NOT NULL, genre_id TEXT NOT NULL,
                PRIMARY KEY (source_id, book_id, genre_id),
                FOREIGN KEY (source_id, book_id) REFERENCES books (source_id, id) ON DELETE CASCADE,
                FOREIGN KEY (genre_id) REFERENCES genres (id) ON DELETE CASCADE);",
            )
            .await?;

        self.conn
            .execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_books_library ON books(in_library);
             CREATE INDEX IF NOT EXISTS idx_novels_format ON novels(format);
             CREATE INDEX IF NOT EXISTS idx_sources_repo ON sources(repo_id);",
            )
            .await?;

        Ok(())
    }

    async fn seed_local_source(&self) -> Result<(), libsql::Error> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sources (id, repo_id, url, icon_url, cover_url_pattern, name, description, config, default_format)
             VALUES ('local', NULL, '', NULL, '', 'Local Library', 'Locally imported books', '{}', 'epub')", ()
        ).await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Repositories
    // ------------------------------------------------------------------
    pub async fn save_repository(&self, repo: &Repository) -> Result<(), libsql::Error> {
        let repo = repo.clone();
        self.conn.execute(
            "INSERT OR REPLACE INTO repositories (id, url, display_name, last_synced_commit, last_checked_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo.id, repo.url, repo.display_name, repo.last_synced_commit, repo.last_checked_timestamp],
        ).await?;
        Ok(())
    }

    pub async fn get_repository(&self, id: &str) -> Result<Option<Repository>, libsql::Error> {
        let mut rows = self.conn.query(
            "SELECT id, url, display_name, last_synced_commit, last_checked_timestamp FROM repositories WHERE id = ?1",
            params![id.to_string()],
        ).await?;

        if let Some(row) = rows.next().await? {
            return Ok(Some(Repository {
                id: row.get(0)?,
                url: row.get(1)?,
                display_name: row.get(2)?,
                last_synced_commit: row.get(3)?,
                last_checked_timestamp: row.get(4)?,
            }));
        }
        Ok(None)
    }

    // ------------------------------------------------------------------
    // Books — private helpers
    // ------------------------------------------------------------------
    fn parse_book_format(s: &str) -> BookFormat {
        match s {
            "mobi" => BookFormat::Mobi,
            "pdf" => BookFormat::Pdf,
            _ => BookFormat::Epub,
        }
    }

    fn book_format_str(f: &BookFormat) -> &'static str {
        match f {
            BookFormat::Epub => "epub",
            BookFormat::Mobi => "mobi",
            BookFormat::Pdf => "pdf",
        }
    }

    fn source_format(f: &SourceType) -> &'static str {
        match f {
            SourceType::WebNovel => "web_novel",
            SourceType::Novel => "novel",
        }
    }

    /// Build a Book from a libsql Row.
    /// Column order: 0:id, 1:source_id, 2:in_library, 3:title, 4:author,
    /// 5:cover_url, 6:rating, 7:status, 8:chapters_count, 9:summary, 10:last_synced,
    /// 11:last_read_timestamp, 12:chapters_path, 13:format, 14:file_path, 15:progress

    fn row_to_book(row: &libsql::Row) -> Result<Book, libsql::Error> {
        let base = BaseBook {
            id: row.get(0)?,
            source_id: row.get(1)?,
            in_library: row.get(2)?,
            title: row.get::<Option<String>>(3)?.unwrap_or_default(),
            author: row.get::<Option<String>>(4)?.unwrap_or_default(),
            cover_url: row.get::<Option<String>>(5)?.unwrap_or_default(),
            rating: row.get::<Option<f64>>(6)?.unwrap_or(0.0) as f32,
            status: row.get::<Option<String>>(7)?.unwrap_or_default(),
            genres: Vec::new(), // Populated out-of-band by query wrappers
            summary: row.get::<Option<String>>(9)?.unwrap_or_default(),
            last_synced: row.get::<Option<i64>>(10)?,
            last_read_timestamp: row.get::<Option<i64>>(11)?.unwrap_or(0),
        };

        if let Some(format_str) = row.get::<Option<String>>(13)? {
            let format = Self::parse_book_format(&format_str);
            Ok(Book::Novel(Novel {
                base,
                format,
                file_path: row.get::<Option<String>>(14)?,
                progress: row.get::<Option<f64>>(15)?.unwrap_or(0.0) as f32,
            }))
        } else {
            Ok(Book::WebNovel(WebNovel {
                base,
                chapters_count: row.get::<Option<i64>>(8)?.unwrap_or(0) as i32,
                chapters_path: row
                    .get::<Option<String>>(12)
                    .unwrap_or_default()
                    .unwrap_or_default(),
                chapters: Vec::new(),
            }))
        }
    }

    async fn fetch_genres_for_book(
        &self,
        book_id: &str,
        source_id: &str,
    ) -> Result<Vec<String>, libsql::Error> {
        let mut rows = self
            .conn
            .query(
                "SELECT g.name FROM genres g JOIN book_genres bg ON g.id = bg.genre_id
             WHERE bg.source_id = ?1 AND bg.book_id = ?2",
                params![source_id.to_string(), book_id.to_string()],
            )
            .await?;

        let mut genres = Vec::new();
        while let Some(row) = rows.next().await? {
            genres.push(row.get::<String>(0)?);
        }
        Ok(genres)
    }

    async fn fetch_chapters_for_book(
        &self,
        book_id: &str,
        source_id: &str,
    ) -> Result<Vec<Chapter>, libsql::Error> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, title, file_path, date, progress, last_read FROM chapters
             WHERE source_id = ?1 AND book_id = ?2 ORDER BY date ASC, id ASC",
                params![source_id.to_string(), book_id.to_string()],
            )
            .await?;

        let mut chapters = Vec::new();
        while let Some(row) = rows.next().await? {
            chapters.push(Chapter {
                id: row.get(0)?,
                title: row.get::<Option<String>>(1)?.unwrap_or_default(),
                file_path: row.get::<Option<String>>(2)?,
                date: row.get(3)?,
                progress: row.get::<Option<f64>>(4)?.unwrap_or(0.0) as f32,
                last_read: row.get::<Option<i64>>(5)?.unwrap_or(0),
            });
        }
        Ok(chapters)
    }

    fn books_select_sql(where_clause: &str) -> String {
        format!(
            "SELECT b.id, b.source_id, b.in_library, b.title, b.author,
                    b.cover_url, b.rating, b.status, COALESCE(w.chapters_count, 0) AS chapters_count,
                    b.summary, b.last_synced, b.last_read_timestamp,
                    w.chapters_path, n.format, n.file_path, COALESCE(n.progress, 0.0) AS progress
             FROM books b
             LEFT JOIN novels n ON n.source_id = b.source_id AND n.book_id = b.id
             LEFT JOIN webnovels w ON w.source_id = b.source_id AND w.book_id = b.id
             {where_clause}"
        )
    }

    // ------------------------------------------------------------------
    // Books — public API
    // ------------------------------------------------------------------

    pub async fn save_book(&self, book: &Book) -> Result<(), libsql::Error> {
        self.conn.execute_batch("BEGIN IMMEDIATE;").await?;

        let result = async {
            let base = book.base();
            let last_synced = if base.source_id == "local" {
                None
            } else {
                Some(base.last_synced.unwrap_or_else(|| Utc::now().timestamp()))
            };

            // 1. Core Book Upsert
            self.conn.execute(
                "INSERT OR REPLACE INTO books (id, source_id, in_library, title, author,
                    cover_url, rating, status, summary, last_synced, last_read_timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    base.id.clone(),
                    base.source_id.clone(),
                    base.in_library,
                    base.title.clone(),
                    base.author.clone(),
                    base.cover_url.clone(),
                    base.rating as f64,
                    base.status.clone(),
                    base.summary.clone(),
                    last_synced,
                    base.last_read_timestamp
                ],
            ).await?;

            // 2. Clear variant branching via match pattern matching
            match book {
                Book::Novel(novel) => {
                    let format_str = Self::book_format_str(&novel.format);
                    self.conn.execute(
                        "INSERT OR REPLACE INTO novels (book_id, source_id, format, file_path, progress) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            novel.base.id.clone(),
                            novel.base.source_id.clone(),
                            format_str.to_string(),
                            novel.file_path.clone(),
                            novel.progress as f64
                        ],
                    ).await?;
                }
                Book::WebNovel(webnovel) => {
                    let chapters_path = if webnovel.chapters_path.is_empty() {
                        storage::webnovel_dir(&webnovel.base.source_id, &webnovel.base.id).to_string_lossy().into_owned()
                    } else {
                        webnovel.chapters_path.clone()
                    };

                    let _ = std::fs::create_dir_all(&chapters_path);

                    self.conn.execute(
                        "INSERT OR REPLACE INTO webnovels (book_id, source_id, chapters_count, chapters_path) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            webnovel.base.id.clone(),
                            webnovel.base.source_id.clone(),
                            webnovel.chapters_count as i64,
                            chapters_path.clone()
                        ],
                    ).await?;

                    for chapter in &webnovel.chapters {
                        let file_path = chapter.file_path.clone().unwrap_or_else(|| {
                            storage::webnovel_chapter_html_path(&webnovel.base.source_id, &webnovel.base.id, &chapter.id)
                                .to_string_lossy()
                                .into_owned()
                        });
                        self.conn.execute(
                            "INSERT OR REPLACE INTO chapters (id, book_id, source_id, title, file_path, date, progress, last_read) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                            params![
                                chapter.id.clone(),
                                webnovel.base.id.clone(),
                                webnovel.base.source_id.clone(),
                                chapter.title.clone(),
                                file_path,
                                chapter.date,
                                chapter.progress as f64,
                                chapter.last_read
                            ],
                        ).await?;
                    }
                }
            }

            // 3. Sync Genres
            for genre_name in &base.genres {
                let genre_id = genre_name.clone();
                self.conn.execute("INSERT OR IGNORE INTO genres (id, name) VALUES (?1, ?2)", params![genre_id.clone(), genre_name.clone()]).await?;
                self.conn.execute("INSERT OR IGNORE INTO book_genres (book_id, source_id, genre_id) VALUES (?1, ?2, ?3)", params![base.id.clone(), base.source_id.clone(), genre_id]).await?;
            }

            Ok(())
        }.await;

        if result.is_err() {
            let _ = self.conn.execute_batch("ROLLBACK;").await;
        } else {
            self.conn.execute_batch("COMMIT;").await?;
        }
        result
    }

    pub async fn get_book(&self, id: &str, source_id: &str) -> Result<Option<Book>, libsql::Error> {
        let mut rows = self
            .conn
            .query(
                &Self::books_select_sql("WHERE b.id = ?1 AND b.source_id = ?2"),
                params![id.to_string(), source_id.to_string()],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            // 1. Reconstruct the base variant from row data
            let mut book = Self::row_to_book(&row)?;

            // 2. Fetch shared relational data (genres)
            let genres = self.fetch_genres_for_book(id, source_id).await?;

            // 3. Inject genres and chapters safely using pattern matching
            match book {
                Book::Novel(ref mut novel) => {
                    novel.base.genres = genres;
                }
                Book::WebNovel(ref mut webnovel) => {
                    webnovel.base.genres = genres;
                    // Fetch and populate chapters exclusively for the WebNovel variant
                    webnovel.chapters = self.fetch_chapters_for_book(id, source_id).await?;
                }
            }

            return Ok(Some(book));
        }
        Ok(None)
    }
    pub async fn get_library_books(&self) -> Result<Vec<Book>, libsql::Error> {
        self.query_books(&Self::books_select_sql(
            "WHERE b.in_library = 1 ORDER BY b.last_read_timestamp DESC",
        ))
        .await
    }

    pub async fn get_all_books(&self) -> Result<Vec<Book>, libsql::Error> {
        self.query_books(&Self::books_select_sql("")).await
    }

    async fn query_books(&self, sql: &str) -> Result<Vec<Book>, libsql::Error> {
        let mut rows = self.conn.query(sql, ()).await?;
        let mut books = Vec::new();
        while let Some(row) = rows.next().await? {
            books.push(Self::row_to_book(&row)?);
        }

        // FIX: Batch fetch all relations (genres and chapters) in one go
        self.populate_book_relations(&mut books).await?;
        Ok(books)
    }

    /// Highly optimized batch fetcher using a single temporary table to avoid N+1 queries
    async fn populate_book_relations(&self, books: &mut [Book]) -> Result<(), libsql::Error> {
        if books.is_empty() {
            return Ok(());
        }

        let mut webnovel_keys = Vec::new();
        let mut all_keys = Vec::new();

        for book in books.iter() {
            let key = (book.source_id().to_string(), book.id().to_string());
            all_keys.push(key.clone());
            if book.is_webnovel() {
                webnovel_keys.push(key);
            }
        }

        self.conn
            .execute_batch(
                "CREATE TEMP TABLE IF NOT EXISTS temp_book_keys (source_id TEXT, book_id TEXT);",
            )
            .await?;
        self.conn
            .execute_batch("DELETE FROM temp_book_keys;")
            .await?;

        for (source_id, book_id) in &all_keys {
            self.conn
                .execute(
                    "INSERT INTO temp_book_keys (source_id, book_id) VALUES (?1, ?2)",
                    params![source_id.as_str(), book_id.as_str()],
                )
                .await?;
        }

        // 1. Fetch all Genres
        let mut genre_rows = self
            .conn
            .query(
                "SELECT bg.source_id, bg.book_id, g.name FROM book_genres bg
             JOIN genres g ON bg.genre_id = g.id
             JOIN temp_book_keys t ON bg.source_id = t.source_id AND bg.book_id = t.book_id",
                (),
            )
            .await?;

        let mut genres_map: HashMap<(String, String), Vec<String>> = HashMap::new();
        while let Some(row) = genre_rows.next().await? {
            let key = (row.get::<String>(0)?, row.get::<String>(1)?);
            genres_map
                .entry(key)
                .or_default()
                .push(row.get::<String>(2)?);
        }

        // 2. Fetch all Chapters (if any webnovels exist)
        let mut chapters_map: HashMap<(String, String), Vec<Chapter>> = HashMap::new();
        if !webnovel_keys.is_empty() {
            let mut chapter_rows = self.conn.query(
                "SELECT c.source_id, c.book_id, c.id, c.title, c.file_path, c.date, c.progress, c.last_read
                 FROM chapters c JOIN temp_book_keys t ON c.source_id = t.source_id AND c.book_id = t.book_id
                 ORDER BY c.date ASC, c.id ASC", ()
            ).await?;

            while let Some(row) = chapter_rows.next().await? {
                let key = (row.get::<String>(0)?, row.get::<String>(1)?);
                chapters_map.entry(key).or_default().push(Chapter {
                    id: row.get(2)?,
                    title: row.get::<Option<String>>(3)?.unwrap_or_default(),
                    file_path: row.get::<Option<String>>(4)?,
                    date: row.get(5)?,
                    progress: row.get::<Option<f64>>(6)?.unwrap_or(0.0) as f32,
                    last_read: row.get::<Option<i64>>(7)?.unwrap_or(0),
                });
            }
        }

        self.conn
            .execute_batch("DROP TABLE IF EXISTS temp_book_keys;")
            .await?;

        // 3. Apply relations back to the Book structs
        for book in books.iter_mut() {
            let key = (book.source_id().to_string(), book.id().to_string());
            if let Some(genres) = genres_map.remove(&key) {
                book.base_mut().genres = genres;
            }
            match book {
                Book::WebNovel(book) => {
                    if let Some(chapters) = chapters_map.remove(&key) {
                        book.chapters = chapters;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
    pub async fn set_in_library(
        &self,
        book_id: &str,
        source_id: &str,
        in_library: bool,
    ) -> Result<(), libsql::Error> {
        self.conn
            .execute(
                "UPDATE books SET in_library = ?1 WHERE id = ?2 AND source_id = ?3",
                params![in_library, book_id.to_string(), source_id.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_book(&self, id: &str, source_id: &str) -> Result<u64, libsql::Error> {
        self.conn
            .execute(
                "DELETE FROM books WHERE id = ?1 AND source_id = ?2",
                params![id.to_string(), source_id.to_string()],
            )
            .await
    }

    // --- Progress Tracking ---
    pub async fn update_chapter_progress(
        &self,
        book_id: &str,
        source_id: &str,
        chapter_id: &str,
        progress: f32,
    ) -> Result<(), libsql::Error> {
        let timestamp = Utc::now().timestamp();
        self.conn.execute("UPDATE chapters SET progress = ?1, last_read = ?2 WHERE source_id = ?3 AND book_id = ?4 AND id = ?5",
            params![progress as f64, timestamp, source_id, book_id, chapter_id]).await?;

        // FIX: Update book timestamp unconditionally on any progress
        self.conn
            .execute(
                "UPDATE books SET last_read_timestamp = ?1 WHERE id = ?2 AND source_id = ?3",
                params![timestamp, book_id, source_id],
            )
            .await?;
        Ok(())
    }

    pub async fn update_book_progress(
        &self,
        book_id: &str,
        source_id: &str,
        progress: f32,
    ) -> Result<(), libsql::Error> {
        let timestamp = Utc::now().timestamp();
        self.conn
            .execute(
                "UPDATE novels SET progress = ?1 WHERE book_id = ?2 AND source_id = ?3",
                params![progress as f64, book_id, source_id],
            )
            .await?;
        self.conn
            .execute(
                "UPDATE books SET last_read_timestamp = ?1 WHERE id = ?2 AND source_id = ?3",
                params![timestamp, book_id, source_id],
            )
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Sources
    // ------------------------------------------------------------------
    pub async fn save_source_with_repo(
        &self,
        source: &SourceWithConfig,
        repo_id: Option<&str>,
    ) -> Result<(), libsql::Error> {
        let source = source.clone();
        let config_json = serde_json::to_string(&source.config).unwrap_or_default();
        let default_format = Self::source_format(&source.config.default_format);

        self.conn.execute(
            "INSERT OR REPLACE INTO sources (id, repo_id, url, cover_url_pattern, name, icon_url, description, config, default_format)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![source.source.id, repo_id.map(str::to_string), source.source.url,
                source.source.cover_url_pattern, source.source.name, source.source.icon_url,
                source.source.description, config_json, default_format.to_string()],
        ).await?;
        Ok(())
    }

    pub async fn save_chapters(
        &self,
        book_id: &str,
        source_id: &str,
        chapters: &[Chapter],
    ) -> Result<(), libsql::Error> {
        self.conn.execute_batch("BEGIN IMMEDIATE;").await?;
        let result = async {
            for chapter in chapters {
                self.conn.execute(
                    "INSERT OR REPLACE INTO chapters (id, book_id, source_id, title, date, file_path, progress, last_read)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        chapter.id.clone(),
                        book_id.to_string(),
                        source_id.to_string(),
                        chapter.title.clone(),
                        chapter.date,
                        chapter.file_path.clone(),
                        chapter.progress as f64,
                        chapter.last_read
                    ],
                ).await?;
            }
            Ok(())
        }.await;

        if result.is_ok() {
            self.conn.execute("COMMIT;", ()).await?;
        } else {
            let _ = self.conn.execute("ROLLBACK;", ()).await;
        }
        result
    }

    pub async fn get_sources(&self) -> Result<Vec<SourceWithConfig>, libsql::Error> {
        let mut rows = self.conn.query("SELECT id, url, cover_url_pattern, name, icon_url, description, config FROM sources", ()).await?;
        let mut sources = Vec::new();
        while let Some(row) = rows.next().await? {
            let config_json: String = row.get::<Option<String>>(6)?.unwrap_or_default();
            sources.push(SourceWithConfig {
                source: Source {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    cover_url_pattern: row.get(2)?,
                    name: row.get(3)?,
                    icon_url: row.get(4)?,
                    description: row.get(5)?,
                },
                config: serde_json::from_str(&config_json).unwrap_or_default(),
            });
        }
        Ok(sources)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::*;

    #[tokio::test]
    async fn test_database_schema() {
        let _database = Database::new(DatabaseMode::Local {
            path: "Library.db".to_string(),
        })
        .await;
    }
}
