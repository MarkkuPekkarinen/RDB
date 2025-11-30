# CLI Reference

Complete reference for all RDB command-line interface commands.

---

## Table of Contents

1. [Installation](#installation)
2. [Global Options](#global-options)
3. [Database Commands](#database-commands)
4. [User Management](#user-management)
5. [Configuration Management](#configuration-management)
6. [Access Control](#access-control)
7. [Server Commands](#server-commands)
8. [Examples](#examples)

---

## Installation

```bash
# Build from source
cargo build --release

# Binary location
./target/release/rdb

# Add to PATH (optional)
export PATH=$PATH:$(pwd)/target/release
```

---

## Global Options

All commands support these global options:

```bash
rdb [OPTIONS] <COMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
```

---

## Database Commands

### `rdb init`

Initialize RDB environment (first-time setup).

```bash
rdb init [OPTIONS]

OPTIONS:
    --force          Force re-initialization (WARNING: May overwrite data)
    -h, --help       Print help information
```

**What it does:**

- Creates `.rdb` directory structure
- Generates default `config.toml`
- Creates `main` database
- Prompts for admin user creation

**Example:**

```bash
$ rdb init
Initializing RDB...
Found 0 existing database(s)
Creating 'main' database...
✓ Created database: main

═══════════════════════════════════════════
  FIRST-TIME SETUP
═══════════════════════════════════════════

Would you like to create an admin user now? (y/n): y
Enter username: admin

✓ Initialization complete!
  Run 'rdb start' to launch the server
  Run 'rdb --help' for more commands
```

### `rdb start`

Start the RDB server.

```bash
rdb start [OPTIONS]

OPTIONS:
    --listen <ADDRESS>    Override listen address (e.g., 0.0.0.0)
    --port <PORT>         Override port number
    --silent              Disable console logging
    -h, --help            Print help information
```

**Examples:**

```bash
# Start with defaults (127.0.0.1:8080)
rdb start

# Listen on all interfaces
rdb start --listen 0.0.0.0

# Custom port
rdb start --port 9090

# Silent mode (logs to file only)
rdb start --silent
```

### `rdb status`

Display comprehensive RDB status.

```bash
rdb status [OPTIONS]

OPTIONS:
    -h, --help       Print help information
```

**Output:**

```
╔════════════════════════════════════════════════════════╗
║              RDB DATABASE STATUS                      ║
╚════════════════════════════════════════════════════════╝

Version: 0.1.0
Config Path: "~/.rdb/config/config.toml"
Root Directory: "~/.rdb"

📊 Configuration:
  Server: 127.0.0.1:8080
  Buffer Pool: 500 pages (2 MB)
  Query Cache: 1000 entries (enabled)

💾 Databases:
  • main (245 KB)
    Path: "~/.rdb/databases/main.db"

  Total: 1 database(s)

═══════════════════════════════════════════════════════
Run 'rdb --help' for available commands
```

### `rdb db`

Database management commands.

```bash
rdb db <SUBCOMMAND>

SUBCOMMANDS:
    create <NAME>    Create a new database
    list             List all databases
    drop <NAME>      Drop a database (coming soon)
    help             Print this message
```

**Examples:**

```bash
# Create new database
rdb db create analytics

# List all databases
rdb db list
```

---

## User Management

### `rdb user add`

Create a new user.

```bash
rdb user add <USERNAME> [OPTIONS]

ARGUMENTS:
    <USERNAME>          Username for the new user

OPTIONS:
    --email <EMAIL>     User email address
    --admin             Grant admin privileges
    --database <DB>     Grant access to specific database
    -h, --help          Print help information
```

**Examples:**

```bash
# Basic user creation (prompts for password)
rdb user add alice

# User with email
rdb user add bob --email bob@example.com

# Admin user
rdb user add admin --admin

# User with database access
rdb user add analyst --database analytics
```

**Interactive Flow:**

```bash
$ rdb user add alice
Password: ********
Confirm password: ********
✓ User 'alice' created successfully
```

### `rdb user list`

List all users.

```bash
rdb user list [OPTIONS]

OPTIONS:
    --verbose        Show detailed user information
    -h, --help       Print help information
```

**Example:**

```bash
$ rdb user list
Users:
  - admin (admin@example.com)
  - alice (alice@example.com)
  - bob (bob@example.com)

Total: 3 users
```

### `rdb user password`

Change user password.

```bash
rdb user password <USERNAME>

ARGUMENTS:
    <USERNAME>       Username to change password for
```

**Example:**

```bash
$ rdb user password alice
Current password: ********
New password: ********
Confirm new password: ********
✓ Password changed successfully
```

### `rdb user delete`

Delete a user (coming soon).

```bash
rdb user delete <USERNAME> [OPTIONS]

ARGUMENTS:
    <USERNAME>       Username to delete

OPTIONS:
    --force          Skip confirmation prompt
    -h, --help       Print help information
```

---

## Configuration Management

### `rdb config show`

Display current configuration.

```bash
rdb config show [OPTIONS]

OPTIONS:
    --format <FORMAT>    Output format: text | json | toml
    -h, --help           Print help information
```

**Example:**

```bash
$ rdb config show
RDB Configuration
━━━━━━━━━━━━━━━━━━━━━━━━━

[Server]
  Host: 127.0.0.1
  Port: 8080
  Workers: 4

[Storage]
  Buffer Pool Size: 500 pages (2 MB)
  Page Size: 4096 bytes
  Compression Threshold: 64 bytes

[Cache]
  Query Cache: Enabled
  Cache Size: 1000 entries
  TTL: 300 seconds
...
```

### `rdb config get`

Get a specific configuration value.

```bash
rdb config get <KEY>

ARGUMENTS:
    <KEY>            Configuration key (e.g., buffer_pool_size)
```

**Example:**

```bash
$ rdb config get buffer_pool_size
buffer_pool_size = 500
```

### `rdb config set`

Set a configuration value.

```bash
rdb config set <KEY> <VALUE>

ARGUMENTS:
    <KEY>            Configuration key
    <VALUE>          New value
```

**Examples:**

```bash
# Increase buffer pool size
rdb config set buffer_pool_size 1000

# Change server port
rdb config set port 9090

# Disable query cache
rdb config set enable_query_cache false
```

### `rdb config reload`

Reload configuration from file.

```bash
rdb config reload
```

**Example:**

```bash
$ rdb config reload
✓ Configuration reloaded from file
```

### `rdb config reset`

Reset configuration to defaults.

```bash
rdb config reset [OPTIONS]

OPTIONS:
    --force          Skip confirmation prompt
    -h, --help       Print help information
```

**Example:**

```bash
$ rdb config reset
⚠ This will reset all configuration to defaults.
Continue? (y/n): y
✓ Configuration reset to defaults
```

---

## Access Control

### `rdb access grant`

Grant database access to a user.

```bash
rdb access grant <USERNAME> <DATABASE> <ROLE>

ARGUMENTS:
    <USERNAME>       Username to grant access to
    <DATABASE>       Database name
    <ROLE>           Role: Owner | DbAdmin | ReadWrite | ReadOnly
```

**Examples:**

```bash
# Grant ReadWrite access
rdb access grant alice main ReadWrite

# Grant ReadOnly access
rdb access grant analyst analytics ReadOnly

# Grant Admin access
rdb access grant bob main DbAdmin
```

### `rdb access revoke`

Revoke database access from a user.

```bash
rdb access revoke <USERNAME> <DATABASE>

ARGUMENTS:
    <USERNAME>       Username to revoke access from
    <DATABASE>       Database name
```

**Example:**

```bash
rdb access revoke alice main
✓ Access revoked for alice on main
```

### `rdb access list`

List all access permissions.

```bash
rdb access list [OPTIONS]

OPTIONS:
    --user <USERNAME>        Filter by username
    --database <DATABASE>    Filter by database
    -h, --help               Print help information
```

**Example:**

```bash
$ rdb access list
Access Control List
━━━━━━━━━━━━━━━━━━━━━━━━━

User: admin
  • main → Owner

User: alice
  • main → ReadWrite
  • analytics → ReadOnly

User: bob
  • main → DbAdmin

Total: 3 users, 4 permissions
```

---

## Server Commands

### `rdb shell`

Start interactive shell (coming soon).

```bash
rdb shell [OPTIONS]

OPTIONS:
    --database <DB>     Connect to specific database
    -h, --help          Print help information
```

---

## Examples

### Complete Setup Workflow

```bash
# 1. Initialize RDB
rdb init

# 2. Create users
rdb user add admin --admin
rdb user add alice --email alice@example.com

# 3. Create additional databases
rdb db create analytics
rdb db create staging

# 4. Grant permissions
rdb access grant alice analytics ReadWrite
rdb access grant alice staging ReadOnly

# 5. Configure performance settings
rdb config set buffer_pool_size 2000
rdb config set query_cache_size 5000

# 6. Start server
rdb start

# 7. Check status
rdb status
```

### Development Setup

```bash
# Local development with debug logging
rdb config set logging.level debug
rdb config set auth.enabled false  # ⚠️ Development only!
rdb start --port 3000
```

### Production Setup

```bash
# Secure production configuration
rdb config set server.host 127.0.0.1  # Reverse proxy only
rdb config set auth.token_expiration 3600  # 1 hour tokens
rdb config set buffer_pool_size 5000  # 20 MB cache
rdb start --silent  # Log to file only
```

---

## Environment Variables

RDB supports these environment variables:

```bash
# Config file location
export RDB_CONFIG=./custom_config.toml

# Data directory
export RDB_DATA_DIR=./data

# Log level
export RDB_LOG_LEVEL=debug
```

---

## Help Command

Get help for any command:

```bash
# General help
rdb --help

# Command-specific help
rdb start --help
rdb user --help
rdb config --help
```

---

## Next Steps

- **[Configuration](configuration.md)** - Detailed configuration options
- **[Authentication](authentication.md)** - User and access management
- **[Querying](querying.md)** - JSON query language reference
