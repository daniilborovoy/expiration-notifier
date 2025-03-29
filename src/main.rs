use chrono::{Local, NaiveDate, Utc};
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

// Database setup
const DB_NAME: &str = "token_notifier.db";

const CREATE_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS tokens (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    last_notified TEXT
)";

// Configuration
#[derive(Debug)]
struct Config {
    telegram_bot_token: String,
    telegram_chat_id: String,
    notification_threshold_days: i64,
    check_interval_seconds: u64,
}

// Token struct for database
#[derive(Debug, Serialize, Deserialize)]
struct Token {
    name: String,
    expires_at: String, // ISO 8601 date string
    last_notified: Option<String>,
}

// CLI Commands
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new token to track
    Add { name: String, expires_at: String },
    /// Remove a token from tracking
    Remove { name: String },
    /// List all tracked tokens
    List,
    /// Start the notification daemon
    Daemon,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Initialize database
    let conn = init_db()?;

    // Load configuration
    let config = Config::from_env()?;

    match cli.command {
        Commands::Add { name, expires_at } => {
            add_token(&conn, &name, &expires_at)?;
            println!("Token '{}' added successfully!", name);
        }
        Commands::Remove { name } => {
            remove_token(&conn, &name)?;
            println!("Token '{}' removed successfully!", name);
        }
        Commands::List => {
            list_tokens(&conn)?;
        }
        Commands::Daemon => {
            run_daemon(&conn, &config)?;
        }
    }

    Ok(())
}

// Database functions
fn init_db() -> SqlResult<Connection> {
    let conn = Connection::open(DB_NAME)?;
    conn.execute(CREATE_TABLE_SQL, [])?;
    Ok(conn)
}

fn add_token(conn: &Connection, name: &str, expires_at: &str) -> SqlResult<()> {
    // Validate date format
    NaiveDate::parse_from_str(expires_at, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

    conn.execute(
        "INSERT OR REPLACE INTO tokens (name, expires_at) VALUES (?1, ?2)",
        params![name, expires_at],
    )?;
    Ok(())
}

fn remove_token(conn: &Connection, name: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM tokens WHERE name = ?1", params![name])?;
    Ok(())
}

fn list_tokens(conn: &Connection) -> SqlResult<()> {
    let mut stmt = conn.prepare("SELECT name, expires_at, last_notified FROM tokens")?;
    let token_iter = stmt.query_map([], |row| {
        Ok(Token {
            name: row.get(0)?,
            expires_at: row.get(1)?,
            last_notified: row.get(2)?,
        })
    })?;

    println!("Tracked Tokens:");
    println!("{:<20} {:<15} {}", "Name", "Expires", "Last Notified");
    println!("{}", "-".repeat(50));

    for token in token_iter {
        let token = token?;
        println!(
            "{:<20} {:<15} {}",
            token.name,
            token.expires_at,
            token.last_notified.unwrap_or_else(|| "Never".to_string())
        );
    }

    Ok(())
}

fn get_expiring_tokens(conn: &Connection, threshold_days: i64) -> SqlResult<Vec<Token>> {
    let now = Utc::now().format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT name, expires_at, last_notified FROM tokens 
         WHERE date(expires_at) <= date(?1, '+' || ?2 || ' days')",
    )?;

    let tokens = stmt
        .query_map(params![now, threshold_days], |row| {
            Ok(Token {
                name: row.get(0)?,
                expires_at: row.get(1)?,
                last_notified: row.get(2)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

    Ok(tokens)
}

fn update_last_notified(conn: &Connection, token_name: &str) -> SqlResult<()> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    conn.execute(
        "UPDATE tokens SET last_notified = ?1 WHERE name = ?2",
        params![now, token_name],
    )?;
    Ok(())
}

// Notification functions
impl Config {
    fn from_env() -> Result<Self, Box<dyn Error>> {
        dotenv::dotenv().ok(); // Load .env file if it exists

        Ok(Self {
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .map_err(|_| "TELEGRAM_BOT_TOKEN environment variable not set")?,
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID")
                .map_err(|_| "TELEGRAM_CHAT_ID environment variable not set")?,
            notification_threshold_days: env::var("NOTIFICATION_THRESHOLD_DAYS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .map_err(|_| "NOTIFICATION_THRESHOLD_DAYS must be a number")?,
            check_interval_seconds: env::var("CHECK_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .map_err(|_| "CHECK_INTERVAL_SECONDS must be a number")?,
        })
    }
}
fn send_telegram_notification(config: &Config, message: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.telegram_bot_token
    );

    let params = [
        ("chat_id", config.telegram_chat_id.as_str()),
        ("text", message),
    ];

    client.post(&url).form(&params).send()?;
    Ok(())
}

fn check_and_notify(conn: &Connection, config: &Config) -> SqlResult<()> {
    let expiring_tokens = get_expiring_tokens(conn, config.notification_threshold_days)?;

    for token in expiring_tokens {
        let expires_date = NaiveDate::parse_from_str(&token.expires_at, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let today = Local::now().date_naive();
        let days_remaining = (expires_date - today).num_days();

        let message = if days_remaining <= 0 {
            format!("🚨 Token '{}' has EXPIRED!", token.name)
        } else {
            format!(
                "⚠️ Token '{}' will expire in {} day{}!",
                token.name,
                days_remaining,
                if days_remaining > 1 { "s" } else { "" }
            )
        };

        if let Err(e) = send_telegram_notification(config, &message) {
            eprintln!("Failed to send notification: {}", e);
        } else {
            update_last_notified(conn, &token.name)?;
        }
    }

    Ok(())
}

fn run_daemon(conn: &Connection, config: &Config) -> Result<(), Box<dyn Error>> {
    println!("Starting token expiration notifier daemon...");
    println!("Checking every {} seconds", config.check_interval_seconds);
    println!(
        "Notification threshold: {} days",
        config.notification_threshold_days
    );

    loop {
        if let Err(e) = check_and_notify(conn, config) {
            eprintln!("Error checking tokens: {}", e);
        }

        std::thread::sleep(std::time::Duration::from_secs(
            config.check_interval_seconds,
        ));
    }
}
