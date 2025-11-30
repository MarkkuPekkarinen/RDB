# RDB Configuration Guide

## Overview

RDB uses a centralized `config.toml` file for all system configuration. This file is automatically created with sensible defaults and can be modified at runtime via CLI commands or API endpoints.

---

## Configuration File: `config.toml`

### Auto-Generation

The `config.toml` file is automatically created in two locations:

1. **Project Root**: `./config.toml` - For development and testing
2. **System Config**: `~/.rdb/config/config.toml` - For production use

When you run `rdb init`, the configuration file is automatically generated with default values.

### Default Configuration

```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4  # Number of worker threads

[database]
default_db = "main"
data_dir = "./data"

[storage]
page_size = 4096  # 4 KB pages
buffer_pool_size = 500  # 500 pages = 2 MB cache
compression_threshold = 64  # Compress tuples > 64 bytes

[cache]
enable_query_cache = true
query_cache_size = 1000  # Cache up to 1000 query results
query_cache_ttl = 300  # 5 minutes

[indexing]
btree_node_size = 64
auto_index_primary_keys = true

[performance]
auto_compact = true
compact_threshold = 30  # Compact when <30% free space
max_batch_size = 10000

[auth]
enabled = true
token_expiration = 86400  # 24 hours
argon2_memory_cost = 65536  # 64 MB
argon2_time_cost = 3
argon2_parallelism = 4

[logging]
level = "info"
log_to_file = false
log_file = "./logs/rdb.log"

[limits]
max_result_rows = 100000
max_query_time = 30  # seconds
max_payload_size = 10485760  # 10 MB
```

---

## Configuration Management via CLI

### View Current Configuration

```bash
rdb config show
```

**Output:**

```yaml
Server:
  Host: 127.0.0.1
  Port: 8080
  Workers: 4

Storage:
  Buffer Pool Size: 500 pages (2 MB)
  Page Size: 4096 bytes
  Compression Threshold: 64 bytes

Cache:
  Query Cache: Enabled
  Cache Size: 1000 entries
  TTL: 300 seconds
```

### Get Specific Value

```bash
rdb config get buffer_pool_size
```

**Output:**

```
buffer_pool_size = 500
```

### Set Configuration Value

```bash
# Increase buffer pool size to 1000 pages (4 MB)
rdb config set buffer_pool_size 1000

# Change server port
rdb config set port 9090

# Disable query cache
rdb config set enable_query_cache false
```

### Reload Configuration from File

```bash
rdb config reload
```

Reloads `config.toml` from disk and applies changes to the running server.

### Reset to Defaults

```bash
rdb config reset
```

Resets all configuration to default values.

---

## Configuration Management via API

### Get Current Configuration

```bash
curl http://localhost:8080/api/config
```

**Response:**

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080,
    "workers": 4
  },
  "storage": {
    "page_size": 4096,
    "buffer_pool_size": 500,
    "compression_threshold": 64
  },
  "cache": {
    "enable_query_cache": true,
    "query_cache_size": 1000,
    "query_cache_ttl": 300
  },
  ...
}
```

### Update Configuration (Partial)

```bash
curl -X POST http://localhost:8080/api/config \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token" \
  -d '{
    "buffer_pool_size": 1000,
    "query_cache_size": 2000,
    "port": 9090
  }'
```

**Response:**

```json
{
  "status": "success",
  "message": "Configuration updated"
}
```

### Reload Configuration from File

```bash
curl -X POST http://localhost:8080/api/config/reload \
  -H "Authorization: Bearer your_token"
```

**Response:**

```json
{
  "status": "success",
  "message": "Configuration reloaded from file"
}
```

---

## Configuration Keys Reference

### Server Configuration

| Key              | Type   | Default     | Description         |
| ---------------- | ------ | ----------- | ------------------- |
| `server.host`    | string | "127.0.0.1" | Server bind address |
| `server.port`    | u16    | 8080        | Server port         |
| `server.workers` | usize  | 4           | Worker thread count |

### Storage Configuration

| Key                             | Type  | Default | Description                      |
| ------------------------------- | ----- | ------- | -------------------------------- |
| `storage.page_size`             | usize | 4096    | Page size in bytes               |
| `storage.buffer_pool_size`      | usize | 500     | Number of pages to cache         |
| `storage.compression_threshold` | usize | 64      | Compress tuples larger than this |

### Cache Configuration

| Key                        | Type  | Default | Description                 |
| -------------------------- | ----- | ------- | --------------------------- |
| `cache.enable_query_cache` | bool  | true    | Enable query result caching |
| `cache.query_cache_size`   | usize | 1000    | Max cached queries          |
| `cache.query_cache_ttl`    | u64   | 300     | Cache TTL in seconds        |

### Performance Configuration

| Key                             | Type  | Default | Description              |
| ------------------------------- | ----- | ------- | ------------------------ |
| `performance.auto_compact`      | bool  | true    | Auto-compact pages       |
| `performance.compact_threshold` | u8    | 30      | Compact threshold (%)    |
| `performance.max_batch_size`    | usize | 10000   | Max batch operation size |

---

## Dynamic Configuration Loading

### Build-Time Configuration

When you build RDB with `cargo build`, it uses the `config.toml` in the project root (if it exists) or creates one with defaults.

### Runtime Configuration

1. **On Init**: `rdb init` creates `config.toml` with defaults if it doesn't exist
2. **On Start**: `rdb start` loads configuration from:
   - `./config.toml` (current directory)
   - `~/.rdb/config/config.toml` (system config)
   - Command-line overrides (`--port`, `--listen`)

### Configuration Priority

1. **Command-line flags** (highest priority)
2. **Environment variables** (if set)
3. **config.toml file** (system or local)
4. **Built-in defaults** (lowest priority)

---

## Examples

### Example 1: Development Setup

```bash
# Initialize with defaults
rdb init

# Edit config.toml to use localhost only
echo 'host = "127.0.0.1"' >> config.toml

# Start server
rdb start
```

### Example 2: Production Setup

```bash
# Initialize
rdb init

# Increase performance settings
rdb config set buffer_pool_size 2000  # 8 MB
rdb config set query_cache_size 5000
rdb config set workers 8

# Start server on custom port
rdb start --listen 0.0.0.0 --port 9090
```

### Example 3: High-Performance Setup

```toml
[storage]
buffer_pool_size = 5000  # 20 MB cache
compression_threshold = 128

[cache]
enable_query_cache = true
query_cache_size = 10000

[performance]
auto_compact = true
compact_threshold = 20
max_batch_size = 50000

[server]
workers = 16  # More workers for high concurrency
```

### Example 4: Low-Resource Setup

```toml
[storage]
buffer_pool_size = 100  # 400 KB cache
compression_threshold = 32

[cache]
enable_query_cache = false  # Disable to save memory

[performance]
auto_compact = false
max_batch_size = 1000

[server]
workers = 2  # Minimal workers
```

---

## Testing Configuration

All configuration features are tested:

```bash
# Run all tests
cargo test --all

# Run configuration tests
cargo test config

# Run integration tests
cargo test --test integration_tests
```

**Test Coverage:**

- ✅ Config file generation
- ✅ Default value loading
- ✅ Config updates via API
- ✅ Config reload functionality
- ✅ Runtime configuration changes
- ✅ Buffer pool size changes
- ✅ Cache size changes

---

## Troubleshooting

### Config File Not Found

If `config.toml` is missing, run:

```bash
rdb init
```

### Invalid Configuration

If the configuration file is invalid, RDB will:

1. Print an error message
2. Fall back to default values
3. Create a new `config.toml.backup`

### Reset Configuration

To reset to defaults:

```bash
# Via CLI
rdb config reset

# Manually
rm config.toml
rdb init
```

### Check Current Configuration

```bash
# Show all settings
rdb config show

# Get specific value
rdb config get buffer_pool_size
```

---

## Performance Impact

| Setting                      | Performance Impact         | Memory Impact |
| ---------------------------- | -------------------------- | ------------- |
| `buffer_pool_size = 500`     | Good for small datasets    | 2 MB          |
| `buffer_pool_size = 2000`    | Better for medium datasets | 8 MB          |
| `buffer_pool_size = 5000`    | Best for large datasets    | 20 MB         |
| `query_cache_size = 1000`    | Good caching               | ~1-5 MB       |
| `query_cache_size = 10000`   | Excellent caching          | ~10-50 MB     |
| `enable_query_cache = false` | Lowest memory              | 0 MB cache    |

---

## Summary

- ✅ **Auto-Generated**: `config.toml` created automatically on init
- ✅ **Dynamic Loading**: Configuration loaded from file at runtime
- ✅ **CLI Management**: Full config control via `rdb config` commands
- ✅ **API Management**: HTTP API for config updates
- ✅ **Hot Reload**: Apply changes without restart
- ✅ **Defaults**: Sensible defaults for all settings
- ✅ **Tested**: Comprehensive test coverage

RDB's configuration system is **production-ready** and **fully dynamic**! 🚀
