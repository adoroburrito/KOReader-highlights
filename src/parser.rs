use crate::models::{BookData, Highlight};
use chrono::{NaiveDate, NaiveDateTime};
use full_moon::ast::{Expression, Field, LastStmt};
use full_moon::tokenizer::TokenType;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidLua(String),
    MissingTitle(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidLua(details) => {
                write!(f, "Failed to parse Lua: {}", details)
            }
            ParseError::MissingTitle(file) => {
                write!(f, "Book has no title in doc_props: {}", file)
            }
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_metadata(content: &str, source_file: &str) -> Result<BookData, ParseError> {
    let ast = full_moon::parse(content)
        .map_err(|e| ParseError::InvalidLua(format!("{}: {}", source_file, e)))?;

    let mut title: Option<String> = None;
    let mut author: Option<String> = None;
    let mut highlights: Vec<Highlight> = Vec::new();

    // Find the return statement (it's a LastStmt, not a regular Stmt)
    if let Some(last_stmt) = ast.nodes().last_stmt() {
        if let LastStmt::Return(return_stmt) = last_stmt {
            for expr in return_stmt.returns().iter() {
                if let Expression::TableConstructor(table) = expr {
                    // Parse the main table
                    for field in table.fields() {
                        if let Field::ExpressionKey { key, value, .. } = field {
                            let key_name = extract_string_from_expr(key);

                            match key_name.as_deref() {
                                Some("doc_props") => {
                                    if let Expression::TableConstructor(props) = value {
                                        (title, author) = extract_doc_props(props);
                                    }
                                }
                                Some("annotations") => {
                                    if let Expression::TableConstructor(annots) = value {
                                        highlights = extract_annotations(annots);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    let title = title.ok_or_else(|| ParseError::MissingTitle(source_file.to_string()))?;

    Ok(BookData {
        title,
        author: author.unwrap_or_else(|| "Unknown".to_string()),
        highlights,
    })
}

fn extract_doc_props(table: &full_moon::ast::TableConstructor) -> (Option<String>, Option<String>) {
    let mut title = None;
    let mut author = None;

    for field in table.fields() {
        if let Field::ExpressionKey { key, value, .. } = field {
            let key_name = extract_string_from_expr(key);
            let val = extract_string_from_expr(value);

            match key_name.as_deref() {
                Some("title") => title = val,
                Some("authors") => author = val,
                _ => {}
            }
        }
    }

    (title, author)
}

fn extract_annotations(table: &full_moon::ast::TableConstructor) -> Vec<Highlight> {
    let mut highlights = Vec::new();

    for field in table.fields() {
        // Each annotation is [N] = { ... }
        if let Field::ExpressionKey { value, .. } = field {
            if let Expression::TableConstructor(annot) = value {
                if let Some(h) = extract_single_annotation(annot) {
                    highlights.push(h);
                }
            }
        }
    }

    highlights
}

fn extract_single_annotation(table: &full_moon::ast::TableConstructor) -> Option<Highlight> {
    let mut chapter: Option<String> = None;
    let mut page: Option<i32> = None;
    let mut text: Option<String> = None;
    let mut datetime: Option<String> = None;

    for field in table.fields() {
        if let Field::ExpressionKey { key, value, .. } = field {
            let key_name = extract_string_from_expr(key);

            match key_name.as_deref() {
                Some("chapter") => chapter = extract_string_from_expr(value),
                Some("pageno") => page = extract_number_from_expr(value),
                Some("text") => text = extract_string_from_expr(value),
                Some("datetime") => datetime = extract_string_from_expr(value),
                _ => {}
            }
        }
    }

    // text is required
    let text = text?;
    let page = page.unwrap_or(0);
    let datetime = datetime.and_then(|s| parse_datetime(&s))?;

    Some(Highlight {
        chapter,
        page,
        text,
        datetime,
    })
}

fn extract_string_from_expr(expr: &Expression) -> Option<String> {
    if let Expression::String(token) = expr {
        let token_type = token.token().token_type();
        if let TokenType::StringLiteral { literal, .. } = token_type {
            return Some(literal.to_string());
        }
    }
    None
}

fn extract_number_from_expr(expr: &Expression) -> Option<i32> {
    if let Expression::Number(token) = expr {
        let token_type = token.token().token_type();
        if let TokenType::Number { text } = token_type {
            return text.parse().ok();
        }
    }
    None
}

fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
}

pub fn filter_by_date(
    highlights: Vec<Highlight>,
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<Highlight> {
    highlights
        .into_iter()
        .filter(|h| {
            let date = h.datetime.date();
            date >= from && date <= to
        })
        .collect()
}

pub fn find_metadata_files(books_path: &Path) -> Vec<PathBuf> {
    WalkDir::new(books_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .map(|n| n.to_string_lossy() == "metadata.epub.lua")
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    const SAMPLE_LUA: &str = r#"
return {
    ["annotations"] = {
        [1] = {
            ["chapter"] = "Chapter 1",
            ["datetime"] = "2026-01-25 10:30:00",
            ["pageno"] = 42,
            ["text"] = "This is a highlighted text",
        },
        [2] = {
            ["chapter"] = "Chapter 2",
            ["datetime"] = "2026-01-26 14:00:00",
            ["pageno"] = 100,
            ["text"] = "Another highlight",
        },
    },
    ["doc_props"] = {
        ["title"] = "Test Book",
        ["authors"] = "Test Author",
    },
}
"#;

    const LUA_WITHOUT_TITLE: &str = r#"
return {
    ["doc_props"] = {
        ["authors"] = "Some Author",
    },
}
"#;

    const LUA_INVALID: &str = r#"
return { this is not valid lua [[[
"#;

    #[test]
    fn test_parse_valid_metadata() {
        let result = parse_metadata(SAMPLE_LUA, "test.lua").unwrap();

        assert_eq!(result.title, "Test Book");
        assert_eq!(result.author, "Test Author");
        assert_eq!(result.highlights.len(), 2);

        let h1 = &result.highlights[0];
        assert_eq!(h1.chapter, Some("Chapter 1".to_string()));
        assert_eq!(h1.page, 42);
        assert_eq!(h1.text, "This is a highlighted text");
    }

    #[test]
    fn test_parse_missing_title() {
        let result = parse_metadata(LUA_WITHOUT_TITLE, "nobook.lua");

        assert!(matches!(result, Err(ParseError::MissingTitle(_))));
    }

    #[test]
    fn test_parse_invalid_lua() {
        let result = parse_metadata(LUA_INVALID, "broken.lua");

        assert!(matches!(result, Err(ParseError::InvalidLua(_))));
    }

    #[test]
    fn test_filter_by_date_includes_range() {
        let book = parse_metadata(SAMPLE_LUA, "test.lua").unwrap();
        let from = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();

        let filtered = filter_by_date(book.highlights, from, to);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "This is a highlighted text");
    }

    #[test]
    fn test_filter_by_date_full_range() {
        let book = parse_metadata(SAMPLE_LUA, "test.lua").unwrap();
        let from = NaiveDate::from_ymd_opt(2026, 1, 25).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 1, 26).unwrap();

        let filtered = filter_by_date(book.highlights, from, to);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_date_empty() {
        let book = parse_metadata(SAMPLE_LUA, "test.lua").unwrap();
        let from = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 2, 10).unwrap();

        let filtered = filter_by_date(book.highlights, from, to);

        assert!(filtered.is_empty());
    }
}
