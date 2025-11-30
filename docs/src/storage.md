# Storage Engine

Deep dive into RDB's storage architecture, page management, and optimization strategies.

---

## Table of Contents

1. [Overview](#overview)
2. [Page-Based Storage](#page-based-storage)
3. [Buffalo Pool](#buffer-pool)
4. [Slotted Pages](#slotted-pages)
5. [B+ Tree Indexing](#b-tree-indexing)
6. [Query Caching](#query-caching)
7. [Compression](#compression)
8. [Performance Tuning](#performance-tuning)

---

## Overview

RDB's storage engine is designed for **high performance** and **space efficiency** with multiple layers of caching and intelligent data organization.

### Storage Hierarchy

```
Query Cache (L1)     ← 100,000 ops/sec
    ↓ miss
Buffer Pool (L2)     ← 10,000 ops/sec
    ↓ miss
B+ Tree Index (L3)   ← 500 ops/sec
    ↓ miss
Disk I/O (L4)        ← 200 ops/sec
```

---

## Page-Based Storage

### Page Structure

All data is stored in **4KB pages**:

```
┌───────────────────── 4096 bytes ─────────────────────┐
│ Page Header (8 bytes)                                │
│ ┌─────────────────────────────────────────────────┐ │
│ │ num_slots | free_space_end | next_page          │ │
│ └─────────────────────────────────────────────────┘ │
│                                                      │
│ Slot Directory (grows downward →)                   │
│ ┌──────┐ ┌──────┐ ┌──────┐                         │
│ │Slot 0│ │Slot 1│ │Slot 2│ ...                     │
│ └──────┘ └──────┘ └──────┘                         │
│                                                      │
│            FREE SPACE                                │
│                                                      │
│ Tuple Data (grows upward ←)                         │
│                    ┌────────┐ ┌────────┐            │
│             ...    │Tuple 1 │ │Tuple 0 │            │
│                    └────────┘ └────────┘            │
└──────────────────────────────────────────────────────┘
```

### Database File Format

```
┌──────────────────────────────────────────┐
│ Page 0: Header Page                      │
│  - File format version                   │
│  - Database name                          │
│  - Page count                             │
│  - Flags                                  │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Page 1: Catalog Page                     │
│  - Table metadata                         │
│  - Column definitions                     │
│  - Index information                      │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Page 2+: Data Pages                      │
│  - Table rows (tuples)                    │
│  - Index nodes                            │
└──────────────────────────────────────────┘
```

---

## Buffer Pool

### LRU Cache Implementation

The buffer pool caches frequently accessed pages in memory using an **LRU (Least Recently Used)** eviction policy.

**Configuration:**

```toml
[storage]
buffer_pool_size = 500  # Number of pages (default: 2 MB)
```

### Cache Performance

| Pool Size  | Memory | Hit Rate | SELECT Latency |
| ---------- | ------ | -------- | -------------- |
| 100 pages  | 400 KB | 60%      | 2 ms           |
| 500 pages  | 2 MB   | 93%      | 200 μs         |
| 1000 pages | 4 MB   | 97%      | 100 μs         |
| 5000 pages | 20 MB  | 99%      | 20 μs          |

### How It Works

```rust
// 1. Request page
let page = buffer_pool.fetch_page(page_id)?;

// 2. Check cache (O(1) lookup)
if cached {
    return page;  // Cache HIT
}

// 3. Load from disk
let page = pager.read_page(page_id)?;

// 4. Add to cache
buffer_pool.insert(page_id, page);

// 5. Evict if full (LRU)
if buffer_pool.is_full() {
    let victim = buffer_pool.evict_lru();
    if victim.is_dirty() {
        pager.write_page(victim)?;
    }
}
```

### Dirty Page Handling

Modified pages are marked "dirty" and flushed to disk:

- **On eviction** - When LRU removes a page
- **On shutdown** - All dirty pages flushed
- **Periodic** - Background flush (optional)

---

## Slotted Pages

### Tuple Storage

Each page uses a **slotted page** layout for efficient tuple storage:

**Features:**

- ✅ Variable-length tuples
- ✅ Space reuse after deletion
- ✅ Automatic compaction
- ✅ Compression for large tuples

### Slot Structure

```rust
struct Slot {
    offset: u16,  // Offset to tuple data
    length: u16,  // Tuple size in bytes
}
```

### Tuple Lifecycle

```
1. INSERT
   ├─ Find free space
   ├─ Add slot entry
   ├─ Write tuple data
   └─ Update page header

2. UPDATE
   ├─ Mark old tuple as deleted
   ├─ Insert new tuple
   └─ Compact if needed

3. DELETE
   ├─ Mark slot as deleted (offset = 0)
   └─ Space reclaimed on compaction

4. COMPACTION
   ├─ Collect active tuples
   ├─ Reset free space pointer
   └─ Rewrite tuples compactly
```

### Automatic Compaction

Triggered when:

- Page runs out of space
- Free space < `compact_threshold` (default: 30%)
- Explicit VACUUM command (planned)

**Configuration:**

```toml
[performance]
auto_compact = true
compact_threshold = 30  # Compact when <30% free
```

---

## B+ Tree Indexing

### Index Structure

RDB uses **B+ trees** for primary key indexing:

```
                   [Root: Internal Node]
                   /         |         \
              [10]          [20]        [30]
             /    \        /    \      /    \
        [Leaf]  [Leaf]  [Leaf]  [Leaf]  [Leaf]  [Leaf]
         1-9    10-19   20-29   30-39   40-49   50-59
```

### Lookup Performance

| Rows       | B+ Tree Depth | Lookups | Full Scan      |
| ---------- | ------------- | ------- | -------------- |
| 100        | 2             | 2 ops   | 100 ops        |
| 10,000     | 3             | 3 ops   | 10,000 ops     |
| 1,000,000  | 4             | 4 ops   | 1,000,000 ops  |
| 10,000,000 | 5             | 5 ops   | 10,000,000 ops |

**Complexity:** O(log N) vs O(N)

### Index Configuration

```toml
[indexing]
btree_node_size = 64           # Keys per node
auto_index_primary_keys = true  # Automatic PK indexing
```

### Index Maintenance

Indexes are automatically maintained on:

- ✅ INSERT - Add key to index
- ✅ DELETE - Remove key from index
- ✅ UPDATE - Update key if primary key changes

---

## Query Caching

### Result Caching

RDB caches SELECT query results for repeated queries:

**Configuration:**

```toml
[cache]
enable_query_cache = true
query_cache_size = 1000  # Number of cached results
query_cache_ttl = 300    # 5 minutes
```

### Cache Key

Queries are hashed based on:

```rust
hash(database + table + columns + where + order_by + limit + offset)
```

### Invalidation

Cache entries are invalidated on:

- ❌ INSERT - Invalidate all queries for table
- ❌ UPDATE - Invalidate all queries for table
- ❌ DELETE - Invalidate all queries for table
- ⏰ TTL - Automatic expiration after configured time

### Performance Impact

| Operation     | Without Cache | With Cache | Speedup |
| ------------- | ------------- | ---------- | ------- |
| Simple SELECT | 5 ms          | **10 μs**  | 500x    |
| Complex query | 50 ms         | **10 μs**  | 5000x   |
| Aggregation   | 200 ms        | **10 μs**  | 20000x  |

---

## Compression

### Automatic Compression

Tuples larger than configured threshold are automatically compressed:

**Configuration:**

```toml
[storage]
compression_threshold = 64  # Compress tuples >64 bytes
```

### Compression Algorithm

RDB uses **Zstd** (Zstandard):

- ✅ Fast compression/decompression
- ✅ High compression ratio (50-87%)
- ✅ Adjustable levels (currently level 3)

### Compression Results

| Tuple Size | Compressed | Savings | CPU Time |
| ---------- | ---------- | ------- | -------- |
| 64 bytes   | No         | 0%      | 0 μs     |
| 128 bytes  | Yes        | 50%     | 10 μs    |
| 1 KB       | Yes        | 70%     | 50 μs    |
| 10 KB      | Yes        | 85%     | 300 μs   |

### When to Use

**Good for:**

- ✅ Large JSON objects
- ✅ Text fields
- ✅ Repeated data

**Not ideal for:**

- ❌ Small tuples (<64 bytes)
- ❌ Already compressed data
- ❌ Random binary data

---

## Performance Tuning

### Memory Configuration

```toml
[storage]
buffer_pool_size = 500  # Start here

# For read-heavy workloads
buffer_pool_size = 2000  # 8 MB

# For large datasets
buffer_pool_size = 10000  # 40 MB
```

### Cache Tuning

```toml
[cache]
enable_query_cache = true

# For repeated queries
query_cache_size = 5000

# For mostly unique queries
query_cache_size = 100
```

### Compression Tuning

```toml
[storage]
# More compression (slower writes, less disk)
compression_threshold = 32

# Less compression (faster writes, more disk)
compression_threshold = 128

# No compression
compression_threshold = 999999
```

### Compaction Tuning

```toml
[performance]
# Aggressive compaction (less space, more CPU)
auto_compact = true
compact_threshold = 20

# Lazy compaction (more space, less CPU)
auto_compact = true
compact_threshold = 50
```

---

## Monitoring

### Check Storage Stats

```bash
# Database size
du -sh ~/.rdb/databases/

# Page count
rdb status
```

### Performance Metrics

```bash
# Cache hit rates (future feature)
curl http://localhost:8080/stats

{
  "buffer_pool": {
    "size": 500,
    "hits": 95000,
    "misses": 5000,
    "hit_rate": 0.95
  },
  "query_cache": {
    "size": 1000,
    "hits": 8500,
    "misses": 1500,
    "hit_rate": 0.85
  }
}
```

---

## Best Practices

1. **Size buffer pool** based on working set

   - 500 pages for small databases
   - 2000+ pages for active datasets

2. **Enable query cache** for read-heavy workloads

   - High cache size for dashboards
   - Low cache size for real-time data

3. **Use compression** for large text/JSON

   - Keep threshold at 64 bytes
   - Adjust based on data patterns

4. **Monitor hit rates**

   - Target 90%+ for buffer pool
   - Target 80%+ for query cache

5. **Compact regularly**
   - Enable auto-compact
   - Set threshold to 30%

---

## Troubleshooting

### High Memory Usage

- Reduce `buffer_pool_size`
- Reduce `query_cache_size`

### Slow Queries

- Increase `buffer_pool_size`
- Ensure indexes on frequently queried columns

### Large Database Files

- Enable compression
- Lower `compression_threshold`
- Run manual compaction (planned)

---

## Next Steps

- **[Performance](performance.md)** - Benchmarks and optimization
- **[Configuration](configuration.md)** - Storage settings
- **[Architecture](architecture.md)** - System overview
