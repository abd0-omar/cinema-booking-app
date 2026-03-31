# cinema-booking

This is a [Gerust](https://gerust.rs) project.

The example application implements a simple tasks management system. [Tasks][db/entities/tasks] are stored in SQLite and can be [created, read, updated, and deleted][web/controllers/tasks] via the web interface. Mutating operations [require a Trailbase-issued JWT][web/middlewares/auth] (`Authorization: Bearer …`): run [Trailbase](https://trailbase.io) as a sidecar, copy its Ed25519 public PEM into config (`trailbase.jwt_public_key_path` or `jwt_public_key_pem`), and obtain tokens from the sidecar’s `/api/auth/v1/*` flows. Integration tests use a fixed bearer token via the `test-helpers` build. See `auth/`, `trailbase-adapter/`, and `TrailbaseAuthConfig` in `config`.

## Prerequisites

* Rust (install via [rustup](https://rustup.rs))
* SQLite 3.x (usually pre-installed on most systems)

## Getting Started

1. Create the database and run migrations:
   ```bash
   cargo db create
   cargo db migrate
   ```

2. Seed the database with initial data:
   ```bash
   cargo db seed
   ```

3. Run the web server:
   ```bash
   cargo run --bin cinema-booking-web
   ```

## Database Commands

The CLI tool provides several database management commands:

* `cargo db create` - Create the SQLite database file
* `cargo db drop` - Delete the database file
* `cargo db migrate` - Apply pending migrations
* `cargo db reset` - Drop, recreate, and migrate the database
* `cargo db seed` - Populate the database with seed data
* `cargo db prepare` - Generate offline query metadata

## SQLite Configuration

This project uses SQLite with optimized PRAGMA settings for performance:

* **WAL mode**: Allows concurrent reads and writes
* **Synchronous NORMAL**: Balanced performance and data safety
* **Busy timeout 5s**: Prevents "database is locked" errors
* **Cache size 20MB**: Improved query performance
* **Foreign keys ON**: Enforces referential integrity
* **Auto vacuum INCREMENTAL**: Gradual space reclaiming
* **Memory temp store**: Faster sorting and indexing
* **MMap size 2GB**: Memory-mapped I/O for faster access
* **Page size 8KB**: Balanced memory usage and performance

## Project Structure

* `config/` - Application configuration
* `auth/` - Auth domain traits and types (`cinema-booking-auth`)
* `trailbase-adapter/` - Trailbase JWT verification and `trailbase-client` helpers (`cinema-booking-trailbase`)
* `db/` - Database entities, migrations, and seeds
* `cli/` - Command-line tools
* `web/` - Web server and API controllers
* `macros/` - Procedural macros

## Testing

Run tests with:
```bash
cargo test --all-features
```

Each test case uses an isolated SQLite database file that is automatically cleaned up after the test completes.

[db/entities/tasks]: ./db/src/entities/tasks.rs
[web/controllers/tasks]: ./web/src/controllers/tasks.rs
[web/middlewares/auth]: ./web/src/middlewares/auth.rs