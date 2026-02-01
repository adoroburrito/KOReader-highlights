use koreader_highlights::config::Config;
use koreader_highlights::db;
use koreader_highlights::parser;
use std::path::Path;

fn main() {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    println!("Books path: {}", config.books_path);
    println!("Database: {}", config.database_path);
    println!("Period: {} to {}", config.from_date, config.to_date);
    println!();

    let conn = match db::init_db(Path::new(&config.database_path)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Database error: {}", e);
            std::process::exit(1);
        }
    };

    let files = parser::find_metadata_files(Path::new(&config.books_path));
    println!("Found {} metadata files", files.len());

    let mut total_found = 0;
    let mut total_inserted = 0;

    for file in files {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read {}: {}", file.display(), e);
                continue;
            }
        };

        let book = match parser::parse_metadata(&content, &file.to_string_lossy()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to parse {}: {}", file.display(), e);
                continue;
            }
        };

        let filtered = parser::filter_by_date(book.highlights, config.from_date, config.to_date);

        if filtered.is_empty() {
            continue;
        }

        println!("\n{} by {}", book.title, book.author);

        for h in &filtered {
            total_found += 1;

            match db::insert_highlight(&conn, h, &book.title, &book.author) {
                Ok(true) => {
                    total_inserted += 1;
                    let preview: String = h.text.chars().take(60).collect();
                    println!("  + p.{}: {}...", h.page, preview);
                }
                Ok(false) => {
                    // duplicate, skip silently
                }
                Err(e) => {
                    eprintln!("  Failed to insert: {}", e);
                }
            }
        }
    }

    println!("\n---");
    println!("Highlights found: {}", total_found);
    println!("New highlights saved: {}", total_inserted);
}
