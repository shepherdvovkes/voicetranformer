# Voice Transformer - Server

[![CI](https://github.com/tokio-rs/tokio/actions/workflows/ci.yml/badge.svg)](https://github.com/tokio-rs/tokio/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

This repository contains the server-side application for the **Voice Transformer** project. This backend is written in Rust and is designed for high performance, reliability, and security, providing the necessary infrastructure for the real-time voice modification clients.

## Architecture Overview

It is crucial to understand that **no real-time audio processing happens on the server**. All DSP and AI-based voice modification is performed client-side to ensure minimal latency.

The server's primary roles are:
1.  **Signaling Server:** Manages the connection setup between users for peer-to-peer (P2P) calls using WebRTC. It helps clients exchange metadata (like network information and session descriptions) so they can establish a direct connection.
2.  **User Authentication & Management:** Handles user registration, login, and profile management.
3.  **Session Management:** Manages call sessions, user presence, and invitations.
4.  **(Future) Asset Delivery:** Will be responsible for delivering AI models, voice presets, and other assets to the clients.
5.  **(Future) Analytics:** Collects anonymized usage data to improve the service.

The entire backend is built as a set of high-performance, asynchronous microservices.

## Technology Stack

* **Language:** [Rust](https://www.rust-lang.org/) (Stable toolchain)
* **Web Framework:** [Actix Web](https://actix.rs/) / [Axum](https://github.com/tokio-rs/axum) - Chosen for its exceptional performance and safety.
* **Asynchronous Runtime:** [Tokio](https://tokio.rs/) - The foundation for all asynchronous operations.
* **Database:** [PostgreSQL](https://www.postgresql.org/) - A robust and reliable open-source relational database.
* **Database Interaction:** [SQLx](https://github.com/launchbadge/sqlx) - A modern, async-ready, and compile-time checked SQL toolkit for Rust.
* **WebSockets:** For real-time signaling communication.
* **Serialization:** [Serde](https://serde.rs/) - For efficient and safe JSON serialization/deserialization.
* **Configuration:** `dotenvy` for managing environment variables.
* **Logging:** `tracing` and `tracing-subscriber`.

## Prerequisites

Before you begin, ensure you have the following installed on your system:
* **Rust Toolchain:** Install via [rustup](https://rustup.rs/).
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
    ```
* **PostgreSQL:** A running instance of PostgreSQL (version 14+).
* **Docker & Docker Compose (Recommended):** The easiest way to run a local PostgreSQL database.
* **`sqlx-cli`:** For managing database migrations.
    ```bash
    cargo install sqlx-cli
    ```

## Getting Started

Follow these steps to get a local development environment running.

### 1. Clone the Repository

```bash
git clone [https://github.com/your-username/voice-transformer-server.git](https://github.com/your-username/voice-transformer-server.git)
cd voice-transformer-server
```

### 2. Set Up the Database

If you have Docker installed, the easiest way to start a database is with Docker Compose.

```bash
# This will start a PostgreSQL container in the background
docker-compose up -d
```

### 3. Configure Environment Variables

Copy the example environment file and update it with your database credentials.

```bash
cp .env.example .env
```

Now, edit the `.env` file. If you used the provided `docker-compose.yml`, the default values should work correctly.

```dotenv
# .env
DATABASE_URL="postgres://postgres:password@localhost:5432/voice_transformer"
# Other settings like JWT secrets, server address, etc.
SERVER_ADDR="127.0.0.1:8080"
JWT_SECRET="your-super-secret-key-that-is-very-long"
```

### 4. Run Database Migrations

With `sqlx-cli` installed and your `DATABASE_URL` set, create and apply the database schema.

```bash
# Create the database specified in DATABASE_URL
sqlx database create

# Run all pending migrations
sqlx migrate run
```
The migration files are located in the `/migrations` directory.

### 5. Build and Run the Server

You can now build and run the application.

```bash
# Build and run in development mode
cargo run

# Or, for production-level performance
cargo run --release
```

The server should now be running on the address specified in your `.env` file (e.g., `http://127.0.0.1:8080`).

## Project Structure

* `.github/workflows/` - CI/CD configuration
* `migrations/` - SQLx database migrations
* `src/` - Source code
    * `api/` - API route handlers (e.g., auth, signaling)
    * `core/` - Core business logic and services
    * `db/` - Database interaction logic
    * `models/` - Data structures and models
    * `config.rs` - Configuration loading
    * `main.rs` - Application entry point
* `.env.example` - Example environment file
* `Cargo.toml` - Rust project manifest
* `docker-compose.yml` - Docker configuration for local dev
* `README.md` - This file

## Contributing

Contributions are welcome! If you'd like to contribute, please fork the repository and create a pull request. For major changes, please open an issue first to discuss what you would like to change.

1.  Fork the Project
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3.  Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4.  Push to the Branch (`git push origin feature/AmazingFeature`)
5.  Open a Pull Request

## License

This project is licensed under the MIT License.
