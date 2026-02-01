use koreader_highlights::config::Config;

fn main() {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    println!("Searching for books in: {}", config.books_path);
    
    let path = std::path::Path::new(&config.books_path);
    let files = koreader_highlights::parser::find_metadata_files(path);
    
    println!("Found {} metadata files", files.len());
    
    let mut total_highlights = 0;
    
    for file in files {
        match std::fs::read_to_string(&file) {
            Ok(content) => {
                match koreader_highlights::parser::parse_metadata(&content, &file.to_string_lossy()) {
                    Ok(book) => {
                        let filtered = koreader_highlights::parser::filter_by_date(
                            book.highlights, 
                            config.from_date, 
                            config.to_date
                        );
                        
                        if !filtered.is_empty() {
                            println!("\nBook: {} by {}", book.title, book.author);
                            for h in &filtered {
                                println!("- {} (Page {}) [{}]", h.text, h.page, h.datetime);
                            }
                            total_highlights += filtered.len();
                        }
                    },
                    Err(e) => eprintln!("Failed to parse {}: {}", file.display(), e),
                }
            },
            Err(e) => eprintln!("Failed to read {}: {}", file.display(), e),
        }
    }
    
    println!("\nTotal highlights found: {}", total_highlights);
}
