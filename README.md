# KOReader Highlights

A helper to extract highlights made on my totally not jailbroken Kindle.

## Why?

I read books. I highlight stuff. I forget what I highlighted. This tool reads the `.sdr` metadata files that [KOReader](https://koreader.rocks/) generates and dumps everything into a SQLite database so I can pretend I'll review my notes later.

## Usage

```bash
# Run on Sunday, get the whole week's highlights (the intended workflow)
koreader-highlights

# Forgot to run it? Get the last 14 days
koreader-highlights --last 14

# Very specific about your dates? Sure
koreader-highlights --from 2026-01-10 --to 2026-01-20

# Kindle mounted somewhere weird? No judgment
koreader-highlights -b /path/to/books -d ./my-highlights.db
```

## Configuration

CLI args > `.env` file > defaults (in that order, because hierarchy matters).

| Option | Env Var | Default |
|--------|---------|---------|
| `-b, --books-path` | `BOOKS_PATH` | `/Volumes/Kindle/livros` |
| `-d, --database-path` | `DATABASE_PATH` | `./highlights.db` |
| `--from` | `FROM_DATE` | Last Sunday |
| `--to` | `TO_DATE` | Yesterday |
| `-l, --last` | - | - |

Create a `.env` file if you're tired of typing the same flags every week like some kind of animal.

## Building

```bash
cargo build --release
```

## License

MIT - do whatever you want, I'm not your mom.
