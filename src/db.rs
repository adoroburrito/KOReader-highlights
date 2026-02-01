use crate::models::Highlight;
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug)]
pub enum DbError {
    ConnectionFailed(String),
    QueryFailed(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::ConnectionFailed(e) => write!(f, "Failed to connect to database: {}", e),
            DbError::QueryFailed(e) => write!(f, "Query failed: {}", e),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        DbError::QueryFailed(e.to_string())
    }
}

pub fn init_db(path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open(path)
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS highlights (
            id INTEGER PRIMARY KEY,
            book_title TEXT NOT NULL,
            book_author TEXT NOT NULL,
            chapter TEXT,
            page INTEGER NOT NULL,
            text TEXT NOT NULL,
            note TEXT,
            datetime TEXT NOT NULL,
            processed INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(book_title, page, text)
        )",
        [],
    )?;

    Ok(conn)
}

pub fn insert_highlight(
    conn: &Connection,
    highlight: &Highlight,
    book_title: &str,
    book_author: &str,
) -> Result<bool, DbError> {
    let datetime_str = highlight.datetime.format("%Y-%m-%d %H:%M:%S").to_string();

    let rows = conn.execute(
        "INSERT OR IGNORE INTO highlights
         (book_title, book_author, chapter, page, text, note, datetime)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            book_title,
            book_author,
            highlight.chapter,
            highlight.page,
            highlight.text,
            highlight.note,
            datetime_str,
        ],
    )?;

    Ok(rows > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn make_highlight(text: &str, page: i32, note: Option<&str>) -> Highlight {
        Highlight {
            chapter: Some("Chapter 1".to_string()),
            page,
            text: text.to_string(),
            note: note.map(String::from),
            datetime: NaiveDateTime::parse_from_str("2026-01-25 10:30:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }

    #[test]
    fn test_init_db_creates_table() {
        let conn = init_db(Path::new(":memory:")).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='highlights'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_insert_highlight() {
        let conn = init_db(Path::new(":memory:")).unwrap();
        let h = make_highlight("Test text", 42, None);

        let inserted = insert_highlight(&conn, &h, "Test Book", "Test Author").unwrap();

        assert!(inserted);

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM highlights", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_insert_highlight_with_note() {
        let conn = init_db(Path::new(":memory:")).unwrap();
        let h = make_highlight("Test text", 42, Some("my note"));

        insert_highlight(&conn, &h, "Test Book", "Test Author").unwrap();

        let note: Option<String> = conn
            .query_row("SELECT note FROM highlights WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(note, Some("my note".to_string()));
    }

    #[test]
    fn test_insert_duplicate_ignored() {
        let conn = init_db(Path::new(":memory:")).unwrap();
        let h = make_highlight("Same text", 42, None);

        let first = insert_highlight(&conn, &h, "Test Book", "Test Author").unwrap();
        let second = insert_highlight(&conn, &h, "Test Book", "Test Author").unwrap();

        assert!(first);
        assert!(!second); // duplicate ignored

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM highlights", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_same_text_different_page_not_duplicate() {
        let conn = init_db(Path::new(":memory:")).unwrap();
        let h1 = make_highlight("Same text", 42, None);
        let h2 = make_highlight("Same text", 100, None);

        insert_highlight(&conn, &h1, "Test Book", "Test Author").unwrap();
        insert_highlight(&conn, &h2, "Test Book", "Test Author").unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM highlights", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 2);
    }
}
