use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use clap::Parser;

const DEFAULT_BOOKS_PATH: &str = "/Volumes/Kindle/livros";
const DEFAULT_DATABASE_PATH: &str = "./highlights.db";

#[derive(Parser, Debug)]
#[command(name = "koreader-highlights")]
#[command(about = "Extract highlights from KOReader metadata files")]
pub struct CliArgs {
    /// Path to the books directory containing .sdr folders
    #[arg(short, long)]
    pub books_path: Option<String>,

    /// Path to the SQLite database file
    #[arg(short, long)]
    pub database_path: Option<String>,

    /// Start date (YYYY-MM-DD)
    #[arg(long)]
    pub from: Option<String>,

    /// End date (YYYY-MM-DD)
    #[arg(long)]
    pub to: Option<String>,

    /// Get highlights from the last N days (mutually exclusive with --from/--to)
    #[arg(short, long)]
    pub last: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub books_path: String,
    pub database_path: String,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
}

#[derive(Debug, PartialEq)]
pub enum ConfigError {
    InvalidDateFormat(String),
    InvalidDateRange,
    MutuallyExclusiveFlags,
    MissingFromDate,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidDateFormat(s) => {
                write!(f, "Invalid date format: '{}'. Expected YYYY-MM-DD", s)
            }
            ConfigError::InvalidDateRange => {
                write!(f, "Invalid date range: --from must be before or equal to --to")
            }
            ConfigError::MutuallyExclusiveFlags => {
                write!(f, "Use --from/--to OR --last, not both")
            }
            ConfigError::MissingFromDate => {
                write!(f, "Use --from together with --to")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();
        let cli = CliArgs::parse();
        Self::from_args(cli, Local::now().date_naive())
    }

    fn from_args(cli: CliArgs, today: NaiveDate) -> Result<Self, ConfigError> {
        let (from_date, to_date) = resolve_dates(&cli, today)?;

        let books_path = cli
            .books_path
            .or_else(|| std::env::var("BOOKS_PATH").ok())
            .unwrap_or_else(|| DEFAULT_BOOKS_PATH.to_string());

        let database_path = cli
            .database_path
            .or_else(|| std::env::var("DATABASE_PATH").ok())
            .unwrap_or_else(|| DEFAULT_DATABASE_PATH.to_string());

        Ok(Config {
            books_path,
            database_path,
            from_date,
            to_date,
        })
    }
}

fn resolve_dates(cli: &CliArgs, today: NaiveDate) -> Result<(NaiveDate, NaiveDate), ConfigError> {
    let has_from_to = cli.from.is_some() || cli.to.is_some();
    let has_last = cli.last.is_some();

    if has_from_to && has_last {
        return Err(ConfigError::MutuallyExclusiveFlags);
    }

    if let Some(days) = cli.last {
        return Ok(compute_last_n_days(today, days));
    }

    if cli.to.is_some() && cli.from.is_none() {
        return Err(ConfigError::MissingFromDate);
    }

    if let Some(ref from_str) = cli.from {
        let from = parse_date(from_str)?;
        let to = match &cli.to {
            Some(to_str) => parse_date(to_str)?,
            None => today - Duration::days(1), // yesterday
        };

        if from > to {
            return Err(ConfigError::InvalidDateRange);
        }

        return Ok((from, to));
    }

    // Default: last Sunday to yesterday
    Ok(compute_week_range(today))
}

fn parse_date(s: &str) -> Result<NaiveDate, ConfigError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| ConfigError::InvalidDateFormat(s.to_string()))
}

fn compute_week_range(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let yesterday = today - Duration::days(1);

    let days_since_sunday = match today.weekday() {
        Weekday::Sun => 7,
        other => other.num_days_from_sunday(),
    };

    let last_sunday = today - Duration::days(days_since_sunday as i64);

    (last_sunday, yesterday)
}

fn compute_last_n_days(today: NaiveDate, days: u32) -> (NaiveDate, NaiveDate) {
    let yesterday = today - Duration::days(1);
    let from = today - Duration::days(days as i64);
    (from, yesterday)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cli(
        from: Option<&str>,
        to: Option<&str>,
        last: Option<u32>,
    ) -> CliArgs {
        CliArgs {
            books_path: None,
            database_path: None,
            from: from.map(String::from),
            to: to.map(String::from),
            last,
        }
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_default_week_range_on_sunday() {
        let cli = make_cli(None, None, None);
        let today = date(2026, 2, 1); // Sunday

        let config = Config::from_args(cli, today).unwrap();

        assert_eq!(config.from_date, date(2026, 1, 25)); // last Sunday
        assert_eq!(config.to_date, date(2026, 1, 31));   // yesterday
    }

    #[test]
    fn test_default_week_range_on_wednesday() {
        let cli = make_cli(None, None, None);
        let today = date(2026, 1, 28); // Wednesday

        let config = Config::from_args(cli, today).unwrap();

        assert_eq!(config.from_date, date(2026, 1, 25)); // last Sunday
        assert_eq!(config.to_date, date(2026, 1, 27));   // yesterday
    }

    #[test]
    fn test_last_n_days() {
        let cli = make_cli(None, None, Some(7));
        let today = date(2026, 2, 1);

        let config = Config::from_args(cli, today).unwrap();

        assert_eq!(config.from_date, date(2026, 1, 25)); // today - 7
        assert_eq!(config.to_date, date(2026, 1, 31));   // yesterday
    }

    #[test]
    fn test_explicit_from_to() {
        let cli = make_cli(Some("2026-01-10"), Some("2026-01-20"), None);
        let today = date(2026, 2, 1);

        let config = Config::from_args(cli, today).unwrap();

        assert_eq!(config.from_date, date(2026, 1, 10));
        assert_eq!(config.to_date, date(2026, 1, 20));
    }

    #[test]
    fn test_only_from_defaults_to_yesterday() {
        let cli = make_cli(Some("2026-01-10"), None, None);
        let today = date(2026, 2, 1);

        let config = Config::from_args(cli, today).unwrap();

        assert_eq!(config.from_date, date(2026, 1, 10));
        assert_eq!(config.to_date, date(2026, 1, 31)); // yesterday
    }

    #[test]
    fn test_only_to_is_error() {
        let cli = make_cli(None, Some("2026-01-20"), None);
        let today = date(2026, 2, 1);

        let result = Config::from_args(cli, today);

        assert_eq!(result, Err(ConfigError::MissingFromDate));
    }

    #[test]
    fn test_last_with_from_is_error() {
        let cli = make_cli(Some("2026-01-10"), None, Some(7));
        let today = date(2026, 2, 1);

        let result = Config::from_args(cli, today);

        assert_eq!(result, Err(ConfigError::MutuallyExclusiveFlags));
    }

    #[test]
    fn test_invalid_date_range() {
        let cli = make_cli(Some("2026-01-20"), Some("2026-01-10"), None);
        let today = date(2026, 2, 1);

        let result = Config::from_args(cli, today);

        assert_eq!(result, Err(ConfigError::InvalidDateRange));
    }

    #[test]
    fn test_default_paths() {
        let cli = make_cli(None, None, None);
        let today = date(2026, 2, 1);

        let config = Config::from_args(cli, today).unwrap();

        assert_eq!(config.books_path, "/Volumes/Kindle/livros");
        assert_eq!(config.database_path, "./highlights.db");
    }
}
