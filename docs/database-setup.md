# Database Setup Guide

This guide explains how to configure Automata to use PostgreSQL for storing workflows and execution data.

## Configuration Methods

The backend uses a configuration hierarchy with the following priority (highest to lowest):

1. Environment variables
2. Environment-specific config files (e.g., `config/app.production.yml`)
3. Main configuration file (`config/app.yml`)
4. Default values

## Quick Start

### 1. Using Environment Variables

The simplest way to configure the database is using environment variables:

```bash
# Set the database URL
export DATABASE_URL="postgres://username:password@localhost:5432/automata"

# Or use the prefixed version
export AUTOMATA_DATABASE__URL="postgres://username:password@localhost:5432/automata"

# Run the application
cargo run
```

### 2. Using Configuration File

Edit `config/app.yml`:

```yaml
database:
  url: "postgres://username:password@localhost:5432/automata"
  max_connections: 20
  min_connections: 5
  connect_timeout: 30
  idle_timeout: 600
  max_lifetime: 1800
```

### 3. Environment-Specific Configuration

Create environment-specific config files:

```bash
# For development
cp config/app.yml config/app.development.yml

# For production
cp config/app.yml config/app.production.yml
```

Then set the environment:

```bash
export APP_ENV=production
cargo run
```

## Database Connection String Format

The database URL follows the standard PostgreSQL connection string format:

```
postgres://[username[:password]@][host][:port][/database][?parameters]
```

Examples:
- Local development: `postgres://postgres:password@localhost:5432/automata`
- Production: `postgres://automata_user:secretpass@db.example.com:5432/automata_prod?sslmode=require`
- Docker: `postgres://postgres:password@postgres:5432/automata`

## Using In-Memory Storage

To use in-memory storage instead of PostgreSQL (useful for testing):

```yaml
database:
  url: "memory"
```

Or:

```bash
export DATABASE_URL="memory"
```

## Verifying Database Connection

When the application starts, you'll see log messages indicating the database connection status:

```
INFO automata: Starting Automata Workflow Engine v0.1.0
INFO automata: Configuration loaded from config/app.yml
INFO automata: Connecting to PostgreSQL database...
INFO automata: Successfully connected to PostgreSQL
```

If the connection fails, the application will fall back to in-memory storage:

```
ERROR automata: Failed to connect to PostgreSQL: connection error. Falling back to in-memory storage.
INFO automata: Using in-memory state manager
```

## Running Migrations

Make sure to run the SQLx migrations after configuring the database:

```bash
# Install SQLx CLI if needed
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run
```

## Additional Configuration Options

The `config/app.yml` file contains many other configuration options:

- **Redis** (optional): For caching and performance optimization
- **Execution Engine**: Concurrency limits, timeouts, checkpointing
- **Security**: JWT settings, CORS origins
- **API**: Request timeouts, body size limits
- **Logging**: Log levels, file output
- **Monitoring**: Metrics and health check endpoints

See the full configuration reference in `config/app.yml`.

## Docker Compose Example

For local development with Docker:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: automata
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

  automata:
    build: .
    environment:
      DATABASE_URL: "postgres://postgres:password@postgres:5432/automata"
    ports:
      - "8080:8080"
    depends_on:
      - postgres

volumes:
  postgres_data:
```

## Troubleshooting

### Connection Refused
- Check if PostgreSQL is running: `pg_isready -h localhost -p 5432`
- Verify the connection string is correct
- Check firewall settings

### Authentication Failed
- Verify username and password
- Check PostgreSQL authentication settings in `pg_hba.conf`
- Ensure the user has proper permissions

### Database Does Not Exist
- Create the database: `createdb automata`
- Or use SQLx: `sqlx database create`

### SSL/TLS Issues
- Add `?sslmode=disable` for local development
- Use `?sslmode=require` for production
- Configure proper certificates for production use
