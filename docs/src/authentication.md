# Authentication & Security

RDB provides enterprise-grade security with JWT authentication, role-based access control, and secure password storage.

---

## Table of Contents

1. [Authentication Overview](#authentication-overview)
2. [User Management](#user-management)
3. [Authorization & Roles](#authorization--roles)
4. [JWT Tokens](#jwt-tokens)
5. [Password Security](#password-security)
6. [API Authentication](#api-authentication)
7. [Best Practices](#best-practices)

---

## Authentication Overview

RDB uses **JWT (JSON Web Tokens)** for stateless authentication. Users authenticate once and receive a token that's valid for a configurable period.

### Authentication Flow

```
1. User sends credentials → Server
2. Server validates credentials
3. Server generates JWT token
4. Server returns token to user
5. User includes token in subsequent requests
6. Server validates token and authorizes access
```

---

## User Management

### Creating Users

#### Via CLI

```bash
# Create a new user (interactive password prompt)
rdb user add alice

# Create user with email
rdb user add bob --email bob@example.com

# Create admin user
rdb user add admin --admin

# List all users
rdb user list
```

#### Via API

```bash
# Login as existing admin first
curl -X POST http://localhost:8080/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin_password"
  }'

# Use the returned token to create a new user
curl -X POST http://localhost:8080/api/users \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "secure_password",
    "role": "ReadWrite"
  }'
```

### User Storage

Users are stored in `~/.rdb/access_control.toml`:

```toml
[[users]]
username = "alice"
email = "alice@example.com"
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$..."

[[users]]
username = "bob"
email = "bob@example.com"
password_hash = "$argon2id$v=19$m=65536,t=3,p=4$..."
```

---

## Authorization & Roles

RDB implements **Role-Based Access Control (RBAC)** with four permission levels:

### Role Hierarchy

| Role          | CREATE | SELECT | INSERT | UPDATE | DELETE | DROP TABLE |
| ------------- | ------ | ------ | ------ | ------ | ------ | ---------- |
| **Owner**     | ✅     | ✅     | ✅     | ✅     | ✅     | ✅         |
| **DbAdmin**   | ✅     | ✅     | ✅     | ✅     | ✅     | ✅         |
| **ReadWrite** | ❌     | ✅     | ✅     | ✅     | ✅     | ❌         |
| **ReadOnly**  | ❌     | ✅     | ❌     | ❌     | ❌     | ❌         |

### Role Descriptions

#### Owner

- **Full database access**
- Can create/drop databases
- Can grant/revoke permissions
- Cannot be removed from owned databases

#### DbAdmin

- **Full table operations**
- Can create/drop tables
- Can modify all data
- Manage database users

#### ReadWrite

- **Data manipulation**
- Can insert, update, delete data
- Cannot modify schema
- Can read all data

#### ReadOnly

- **Read-only access**
- Can only SELECT data
- No modification permissions
- Useful for reporting/analytics

### Granting Permissions

```bash
# Grant ReadWrite access to Alice on 'main' database
rdb access grant alice main ReadWrite

# Grant DbAdmin access to Bob
rdb access grant bob main DbAdmin

# List all access permissions
rdb access list
```

### Per-Database Permissions

Users can have different roles on different databases:

```toml
[[acl]]
username = "alice"
database = "main"
role = "ReadWrite"

[[acl]]
username = "alice"
database = "analytics"
role = "ReadOnly"
```

---

## JWT Tokens

### Token Structure

RDB uses standard JWT tokens with three parts:

```
header.payload.signature
```

**Example:**

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.
eyJzdWIiOiJhbGljZSIsImV4cCI6MTYzODM2MDAwMH0.
SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
```

### Token Payload

```json
{
  "sub": "alice", // Subject (username)
  "exp": 1638360000, // Expiration timestamp
  "iat": 1638273600, // Issued at timestamp
  "role": "ReadWrite", // User role
  "database": "main" // Database scope
}
```

### Token Expiration

Configure token lifetime in `config.toml`:

```toml
[auth]
enabled = true
token_expiration = 86400  # 24 hours in seconds
```

**Common Values:**

- `3600` - 1 hour
- `86400` - 24 hours (default)
- `604800` - 7 days
- `2592000` - 30 days

### Refreshing Tokens

Tokens cannot be refreshed. Users must re-authenticate when tokens expire:

```bash
# Re-login to get a new token
curl -X POST http://localhost:8080/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "password"
  }'
```

---

## Password Security

### Argon2 Hashing

RDB uses **Argon2id** - the winner of the Password Hashing Competition:

```toml
[auth]
argon2_memory_cost = 65536  # 64 MB
argon2_time_cost = 3        # 3 iterations
argon2_parallelism = 4      # 4 threads
```

**Benefits:**

- ✅ **Memory-hard** - Resistant to GPU attacks
- ✅ **Slow by design** - Prevents brute force
- ✅ **Configurable** - Adjust security vs performance
- ✅ **Industry standard** - Recommended by OWASP

### Password Requirements

**Minimum Requirements:**

- At least 8 characters (recommended: 12+)
- Mix of uppercase, lowercase, numbers, symbols (recommended)
- Not in common password list (planned)

### Changing Passwords

```bash
# Change password via CLI
rdb user password alice

# Via API (requires current authentication)
curl -X PUT http://localhost:8080/api/users/alice/password \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "old_password": "current_pass",
    "new_password": "new_secure_pass"
  }'
```

---

## API Authentication

### Login

```bash
curl -X POST http://localhost:8080/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "secure_password"
  }'
```

**Response:**

```json
{
  "status": "success",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

### Authenticated Requests

Include the token in the `Authorization` header:

```bash
curl -X POST http://localhost:8080/query \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "Select": {
      "database": "main",
      "from": "users",
      "columns": ["*"]
    }
  }'
```

### Error Responses

#### Missing Token

```json
{
  "status": "error",
  "message": "Missing Authorization header"
}
```

#### Invalid Token

```json
{
  "status": "error",
  "message": "Invalid token format"
}
```

#### Insufficient Permissions

```json
{
  "status": "error",
  "message": "Insufficient permissions for this operation"
}
```

---

## Best Practices

### 1. Use Strong Passwords

```bash
# Good
rdb user add alice
Password: Tr0ub4dor&3_Complex!Pass

# Bad
rdb user add alice
Password: password123
```

### 2. Principle of Least Privilege

```bash
# Grant minimum required role
rdb access grant analyst main ReadOnly  # ✅ Good

# Don't grant unnecessary permissions
rdb access grant analyst main Owner     # ❌ Bad
```

### 3. Rotate Tokens Regularly

```toml
[auth]
# Use shorter expiration for sensitive data
token_expiration = 3600  # 1 hour
```

### 4. Secure Token Storage

```javascript
// Browser - use httpOnly cookies
document.cookie = "token=...; HttpOnly; Secure; SameSite=Strict";

// Never store in localStorage (XSS vulnerable)
localStorage.setItem("token", "..."); // ❌ Bad
```

### 5. Use HTTPS in Production

```toml
[server]
# Use reverse proxy (nginx/traefik) for HTTPS
host = "127.0.0.1"  # Bind to localhost
port = 8080
```

### 6. Monitor Authentication Logs

```bash
# Check logs for failed logins
tail -f ~/.rdb/log/engine.log | grep "login"
```

### 7. Disable Auth for Development Only

```toml
[auth]
# Only disable for local development
enabled = false  # ⚠️ WARNING: No authentication!
```

---

## Troubleshooting

### "Missing Authorization header"

**Cause:** Token not included in request  
**Solution:** Add `Authorization: Bearer <token>` header

### "Invalid token format"

**Cause:** Malformed token or missing "Bearer" prefix  
**Solution:** Ensure format is `Bearer eyJhbGc...`

### "Token expired"

**Cause:** Token older than configured expiration  
**Solution:** Re-login to get a new token

### "Insufficient permissions"

**Cause:** User role doesn't allow operation  
**Solution:** Request access from database owner or use correct credentials

### "User not found"

**Cause:** Username doesn't exist  
**Solution:** Create user with `rdb user add <username>`

---

## Security Checklist

- [ ] Change default admin password
- [ ] Use strong passwords (12+ characters)
- [ ] Enable HTTPS in production
- [ ] Set appropriate token expiration
- [ ] Grant minimum required permissions
- [ ] Monitor authentication logs
- [ ] Rotate credentials regularly
- [ ] Use firewall rules to restrict access
- [ ] Keep RDB updated
- [ ] Backup `access_control.toml` securely

---

## Next Steps

- **[CLI Reference](cli.md)** - User management commands
- **[Configuration](configuration.md)** - Auth settings
- **[Troubleshooting](troubleshooting.md)** - Common issues
