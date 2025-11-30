# Complete Query Reference

RDB uses a JSON-based query language. All queries are sent as HTTP POST requests to `/query`.

## Table of Contents

- [DDL (Data Definition Language)](#ddl-data-definition-language)
  - [CREATE TABLE](#create-table)
  - [DROP TABLE](#drop-table)
- [DML (Data Manipulation Language)](#dml-data-manipulation-language)
  - [INSERT](#insert)
  - [UPDATE](#update)
  - [DELETE](#delete)
- [DQL (Data Query Language)](#dql-data-query-language)
  - [SELECT](#select)
  - [WHERE Clause](#where-clause)
  - [ORDER BY](#order-by)
  - [LIMIT and OFFSET](#limit-and-offset)
- [Advanced Operations](#advanced-operations)
  - [BATCH](#batch)
- [Complete CRUD Examples](#complete-crud-examples)

---

## DDL (Data Definition Language)

### CREATE TABLE

Creates a new table with the specified columns and constraints.

**JSON Syntax:**

```json
{
  "CreateTable": {
    "database": "main",
    "table": "users",
    "columns": [
      {
        "name": "id",
        "type": "int",
        "primary_key": true,
        "unique": false,
        "nullable": false
      },
      {
        "name": "name",
        "type": "string",
        "unique": false,
        "nullable": true
      },
      {
        "name": "email",
        "type": "string",
        "unique": true,
        "nullable": false
      },
      {
        "name": "age",
        "type": "int",
        "nullable": true
      }
    ]
  }
}
```

**cURL Example:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token_here" \
  -d '{
    "CreateTable": {
      "database": "main",
      "table": "users",
      "columns": [
        {"name": "id", "type": "int", "primary_key": true},
        {"name": "name", "type": "string"}
      ]
    }
  }'
```

### DROP TABLE

Deletes a table and all its data.

**JSON Syntax:**

```json
{
  "DropTable": {
    "database": "main",
    "table": "users"
  }
}
```

**cURL Example:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token_here" \
  -d '{"DropTable": {"database": "main", "table": "users"}}'
```

---

## DML (Data Manipulation Language)

### INSERT

Inserts one or more rows into a table.

**Single Row:**

```json
{
  "Insert": {
    "database": "main",
    "table": "users",
    "values": [
      { "id": 1, "name": "Alice", "email": "alice@example.com", "age": 30 }
    ]
  }
}
```

**Multiple Rows:**

```json
{
  "Insert": {
    "database": "main",
    "table": "users",
    "values": [
      { "id": 1, "name": "Alice", "email": "alice@example.com", "age": 30 },
      { "id": 2, "name": "Bob", "email": "bob@example.com", "age": 25 },
      { "id": 3, "name": "Charlie", "email": "charlie@example.com", "age": 35 }
    ]
  }
}
```

**cURL Example:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token_here" \
  -d '{
    "Insert": {
      "database": "main",
      "table": "users",
      "values": [
        {"id": 1, "name": "Alice", "email": "alice@example.com"}
      ]
    }
  }'
```

### UPDATE

Updates existing rows that match the WHERE clause.

**JSON Syntax:**

```json
{
  "Update": {
    "database": "main",
    "table": "users",
    "set": {
      "name": "Alice Smith",
      "age": 31
    },
    "where": {
      "column": "id",
      "cmp": "=",
      "value": 1
    }
  }
}
```

**Update Multiple Fields:**

```json
{
  "Update": {
    "database": "main",
    "table": "users",
    "set": {
      "email": "newemail@example.com",
      "age": 40
    },
    "where": {
      "column": "name",
      "cmp": "=",
      "value": "Bob"
    }
  }
}
```

**cURL Example:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token_here" \
  -d '{
    "Update": {
      "database": "main",
      "table": "users",
      "set": {"name": "Alice Smith"},
      "where": {"column": "id", "cmp": "=", "value": 1}
    }
  }'
```

### DELETE

Deletes rows that match the WHERE clause.

**JSON Syntax:**

```json
{
  "Delete": {
    "database": "main",
    "table": "users",
    "where": {
      "column": "id",
      "cmp": "=",
      "value": 2
    }
  }
}
```

**Delete All Rows (use with caution):**

```json
{
  "Delete": {
    "database": "main",
    "table": "users",
    "where": null
  }
}
```

**cURL Example:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token_here" \
  -d '{
    "Delete": {
      "database": "main",
      "table": "users",
      "where": {"column": "id", "cmp": "=", "value": 2}
    }
  }'
```

---

## DQL (Data Query Language)

### SELECT

Retrieves data from a table.

**Select All:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"]
  }
}
```

**Select Specific Columns:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["name", "email"]
  }
}
```

### WHERE Clause

Filter results using various comparison operators.

**Supported Operators:**

| Operator | Description                     | Example Value             |
| -------- | ------------------------------- | ------------------------- |
| `=`      | Equal to                        | `1`, `"Alice"`            |
| `!=`     | Not equal to                    | `2`                       |
| `>`      | Greater than                    | `18`                      |
| `<`      | Less than                       | `65`                      |
| `>=`     | Greater than or equal           | `21`                      |
| `<=`     | Less than or equal              | `100`                     |
| `LIKE`   | Pattern matching (`%` wildcard) | `"A%"`, `"%@example.com"` |
| `IN`     | Value in list                   | `[1, 2, 3]`               |

**Examples:**

**Equality:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "where": {
      "column": "id",
      "cmp": "=",
      "value": 1
    }
  }
}
```

**Greater Than:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "where": {
      "column": "age",
      "cmp": ">",
      "value": 25
    }
  }
}
```

**LIKE Pattern:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "where": {
      "column": "email",
      "cmp": "LIKE",
      "value": "%@example.com"
    }
  }
}
```

**IN Operator:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "where": {
      "column": "id",
      "cmp": "IN",
      "value": [1, 3, 5]
    }
  }
}
```

### ORDER BY

Sort results by one or more columns.

**Ascending Order (default):**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "order_by": {
      "column": "name",
      "direction": "ASC"
    }
  }
}
```

**Descending Order:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "order_by": {
      "column": "age",
      "direction": "DESC"
    }
  }
}
```

### LIMIT and OFFSET

Paginate results.

**LIMIT (first 10 rows):**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "limit": 10
  }
}
```

**OFFSET (skip first 20 rows):**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "offset": 20,
    "limit": 10
  }
}
```

**Combined with ORDER BY:**

```json
{
  "Select": {
    "database": "main",
    "from": "users",
    "columns": ["*"],
    "where": {
      "column": "age",
      "cmp": ">",
      "value": 18
    },
    "order_by": {
      "column": "name",
      "direction": "ASC"
    },
    "offset": 0,
    "limit": 25
  }
}
```

---

## Advanced Operations

### BATCH

Execute multiple queries in a single request.

**JSON Syntax:**

```json
{
  "Batch": [
    {
      "CreateTable": {
        "database": "main",
        "table": "products",
        "columns": [
          { "name": "id", "type": "int", "primary_key": true },
          { "name": "name", "type": "string" },
          { "name": "price", "type": "float" }
        ]
      }
    },
    {
      "Insert": {
        "database": "main",
        "table": "products",
        "values": [
          { "id": 1, "name": "Laptop", "price": 999.99 },
          { "id": 2, "name": "Mouse", "price": 29.99 }
        ]
      }
    },
    {
      "Select": {
        "database": "main",
        "from": "products",
        "columns": ["*"]
      }
    }
  ]
}
```

**Response:**

```json
[
  "Table products created",
  "Inserted",
  [
    { "id": 1, "name": "Laptop", "price": 999.99 },
    { "id": 2, "name": "Mouse", "price": 29.99 }
  ]
]
```

---

## Complete CRUD Examples

### Full Workflow Example

**1. Create Table:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token" \
  -d '{
    "CreateTable": {
      "database": "main",
      "table": "employees",
      "columns": [
        {"name": "id", "type": "int", "primary_key": true},
        {"name": "name", "type": "string"},
        {"name": "department", "type": "string"},
        {"name": "salary", "type": "float"}
      ]
    }
  }'
```

**2. Insert Data:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token" \
  -d '{
    "Insert": {
      "database": "main",
      "table": "employees",
      "values": [
        {"id": 1, "name": "John", "department": "Engineering", "salary": 75000},
        {"id": 2, "name": "Jane", "department": "Marketing", "salary": 65000},
        {"id": 3, "name": "Bob", "department": "Engineering", "salary": 80000}
      ]
    }
  }'
```

**3. Query Data:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token" \
  -d '{
    "Select": {
      "database": "main",
      "from": "employees",
      "columns": ["*"],
      "where": {
        "column": "department",
        "cmp": "=",
        "value": "Engineering"
      },
      "order_by": {
        "column": "salary",
        "direction": "DESC"
      }
    }
  }'
```

**4. Update Data:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token" \
  -d '{
    "Update": {
      "database": "main",
      "table": "employees",
      "set": {"salary": 85000},
      "where": {"column": "name", "cmp": "=", "value": "Bob"}
    }
  }'
```

**5. Delete Data:**

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your_token" \
  -d '{
    "Delete": {
      "database": "main",
      "table": "employees",
      "where": {"column": "id", "cmp": "=", "value": 2}
    }
  }'
```

## SQL to JSON Mapping

| SQL                                                     | RDB JSON                                                                                                                                                       |
| ------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR)` | `{"CreateTable": {"database": "main", "table": "users", "columns": [{"name": "id", "type": "int", "primary_key": true}, {"name": "name", "type": "string"}]}}` |
| `DROP TABLE users`                                      | `{"DropTable": {"database": "main", "table": "users"}}`                                                                                                        |
| `INSERT INTO users VALUES (1, 'Alice')`                 | `{"Insert": {"database": "main", "table": "users", "values": [{"id": 1, "name": "Alice"}]}}`                                                                   |
| `SELECT * FROM users`                                   | `{"Select": {"database": "main", "from": "users", "columns": ["*"]}}`                                                                                          |
| `SELECT name FROM users WHERE id = 1`                   | `{"Select": {"database": "main", "from": "users", "columns": ["name"], "where": {"column": "id", "cmp": "=", "value": 1}}}`                                    |
| `UPDATE users SET name='Bob' WHERE id=1`                | `{"Update": {"database": "main", "table": "users", "set": {"name": "Bob"}, "where": {"column": "id", "cmp": "=", "value": 1}}}`                                |
| `DELETE FROM users WHERE id=1`                          | `{"Delete": {"database": "main", "table": "users", "where": {"column": "id", "cmp": "=", "value": 1}}}`                                                        |
| `SELECT * FROM users ORDER BY name ASC LIMIT 10`        | `{"Select": {"database": "main", "from": "users", "columns": ["*"], "order_by": {"column": "name", "direction": "ASC"}, "limit": 10}}`                         |
| `SELECT * FROM users WHERE age > 18`                    | `{"Select": {"database": "main", "from": "users", "columns": ["*"], "where": {"column": "age", "cmp": ">", "value": 18}}}`                                     |
| `SELECT * FROM users WHERE email LIKE '%@example.com'`  | `{"Select": {"database": "main", "from": "users", "columns": ["*"], "where": {"column": "email", "cmp": "LIKE", "value": "%@example.com"}}}`                   |

## ACID Compliance

RDB is designed with ACID properties in mind:

- **Atomicity**: Each individual query executes atomically. Batch queries execute all operations or none.
- **Consistency**: Data validation and constraints are enforced at the storage layer.
- **Isolation**: Currently single-threaded execution ensures isolation. Multi-threaded execution with proper locking is planned.
- **Durability**: All changes are persisted to disk. The buffer pool flushes dirty pages automatically.

## Future Features

- **JOINs**: Support for `INNER JOIN`, `LEFT JOIN`, `RIGHT JOIN`, `FULL OUTER JOIN`
- **Transactions**: BEGIN, COMMIT, ROLLBACK for multi-statement transactions
- **GROUP BY & HAVING**: Aggregation queries
- **Aggregate Functions**: COUNT, SUM, AVG, MIN, MAX
- **Subqueries**: Nested SELECT statements
- **Indexes**: Secondary indexes on non-primary-key columns
- **Views**: Virtual tables based on SELECT queries
