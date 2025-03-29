# Token Expiration Notifier 🔔

A Rust application that tracks token expiration dates and sends Telegram notifications when tokens are about to expire.
<div style="width: 100%; display: flex; justify-content: center">
    <img src="assets/logo.jpeg" width="200" style="border-radius: 20px">
</div>

## Features ✨

- 🗃️ Track multiple tokens of any type (GitLab, GitHub, AWS, etc.)
- ⏰ Configurable notification thresholds
- 🔔 Telegram notifications for expiring tokens
- 💾 SQLite database for persistent storage
- 🖥️ Simple CLI interface for management
- ⚙️ Configurable via environment variables

## Installation 📦

### Prerequisites

- Rust 1.60+ (install via [rustup](https://rustup.rs/))
- SQLite development libraries
- Telegram bot token and chat ID

### Steps

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/token-notifier.git
   cd token-notifier
   ```

2. Build the application:
   ```bash
   cargo build --release
   ```

## Configuration ⚙️

Create a `.env` file in the project root:

```env
# Required
TELEGRAM_BOT_TOKEN=your_bot_token_here
TELEGRAM_CHAT_ID=your_chat_id_here

# Optional (defaults shown)
NOTIFICATION_THRESHOLD_DAYS=1
CHECK_INTERVAL_SECONDS=3600
```

## Usage 🚀

### CLI Commands

```bash
# Add a new token to track
./target/release/token-notifier add "GitLab API" "2026-12-31"

# Remove a token
./target/release/token-notifier remove "GitLab API"

# List all tracked tokens
./target/release/token-notifier list

# Start the notification daemon
./target/release/token-notifier daemon
```

### Docker Usage

```bash
docker build -t token-notifier .
docker run -d --env-file .env token-notifier daemon
``` 

## Database Schema 💾

The SQLite database (`token_notifier.db`) contains:

```sql
CREATE TABLE tokens (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,  -- Token name/identifier
    expires_at TEXT NOT NULL,   -- Expiration date (YYYY-MM-DD)
    last_notified TEXT          -- Last notification timestamp
);
```

## Deployment 🚢

### As a Systemd Service

1. Create `/etc/systemd/system/token-notifier.service`:
   ```ini
   [Unit]
   Description=Token Expiration Notifier
   After=network.target

   [Service]
   User=root
   WorkingDirectory=/opt/token-notifier
   ExecStart=/opt/token-notifier/token-notifier daemon
   Restart=always
   EnvironmentFile=/opt/token-notifier/.env

   [Install]
   WantedBy=multi-user.target
   ```

2. Enable and start the service:
   ```bash
   sudo systemctl enable token-notifier
   sudo systemctl start token-notifier
   ```

## Troubleshooting 🐛

### Common Issues

**"Environment variable not found" error**
- Verify `.env` file exists in the working directory
- Check variable names match exactly (case-sensitive)
- For systemd services, ensure `EnvironmentFile` path is correct

**Telegram notifications not working**
- Verify your bot token and chat ID are correct
- Check if the bot has been started with `/start` in your chat
- Ensure the bot has permission to send messages

**Database issues**
- Verify SQLite libraries are installed
- Check write permissions in the application directory

## Contributing 🤝

Contributions are welcome! Please open an issue or PR for any:
- Bug fixes
- New features
- Documentation improvements
