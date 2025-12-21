# Technology Stack: BurnCloud

## Core Language & Runtime
*   **Language:** Rust (Edition 2021)
    *   Reason: Memory safety, zero-cost abstractions, and high performance.
*   **Async Runtime:** Tokio
    *   Reason: Industry standard for async I/O in Rust, offering a robust ecosystem.

## Web & API Layer
*   **Framework:** Axum
    *   Reason: Ergonomic, modular, and built on top of Hyper/Tower for maximum performance.
*   **Middleware:** Tower
    *   Reason: Composable middleware stack for handling timeouts, tracing, and CORS.
*   **HTTP Client:** reqwest
    *   Reason: Powerful, async-native HTTP client for upstream requests.

## User Interface (Desktop)
*   **Framework:** Dioxus
    *   Reason: React-like declarative UI in Rust, cross-platform support, and strong type safety.
*   **Renderer:** Dioxus Desktop (WebView) / Dioxus LiveView
    *   Reason: Enables a native desktop experience using web technologies (HTML/CSS) while keeping logic in Rust.

## Data Persistence
*   **Database:** SQLite
    *   Reason: Serverless, zero-configuration, and perfect for local/embedded deployment.
*   **ORM/Query Builder:** sqlx
    *   Reason: Async, compile-time checked SQL queries for safety and performance.

## Core Libraries & Utilities
*   **Serialization:** Serde / Serde JSON
    *   Reason: The standard for serialization/deserialization in Rust.
*   **Error Handling:** anyhow / thiserror
    *   Reason: Idiomatic error handling (applications vs libraries).
*   **Logging:** Tracing / Env_logger
    *   Reason: Structured logging and diagnostics.
