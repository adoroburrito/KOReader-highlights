use koreader_highlights::config::Config;

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
    println!("From: {}", config.from_date);
    println!("To: {}", config.to_date);
}
