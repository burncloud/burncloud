# Rust Code Style Guide

## 1. General Principles
- **Safety First:** Adhere strictly to the "Zero Unsafe" policy. `unsafe` is only permitted for necessary FFI and must be heavily documented.
- **Idiomatic Rust:** Follow standard Rust conventions (Clippy is your friend).
- **Strict Typing:** Leverage the type system. Avoid "stringly typed" APIs. Use Newtypes (e.g., `struct UserId(String)`) to enforce semantic meaning.

## 2. Formatting & Naming
- **Tooling:** Always use `rustfmt` with default settings.
- **Naming:**
    - `SnakeCase` for crates, modules, functions, methods, and variables.
    - `CamelCase` for types (structs, enums) and traits.
    - `SCREAMING_SNAKE_CASE` for constants and statics.

## 3. Async & Concurrency
- **Runtime:** Use `tokio` for all async runtime needs.
- **Blocking:** NEVER block an async thread. Use `tokio::task::spawn_blocking` for CPU-intensive or synchronous I/O operations.
- **Cancellation:** Ensure async tasks are cancellation-safe where possible.

## 4. Error Handling
- **Application Code:** Use `anyhow::Result` for ease of error propagation in the top-level application logic / binaries.
- **Library Code:** Use `thiserror` to define explicit, structured error types for library crates (`router`, `core`, etc.).
- **Panics:** Avoid panics in production code. Use `Result` or `Option` to handle expected failures.

## 5. Project Structure
- **Modules:** Keep modules small and focused. Expose a clean public API via `pub use`.
- **Visibility:** Default to private. Only make items `pub` if they are part of the stable external API of the crate.

## 6. Testing
- **Unit Tests:** Co-locate unit tests in the same file as the code (in a `tests` module).
- **Integration Tests:** Place integration tests in the `tests/` directory of the crate.
- **Mocking:** Prefer using traits and dependency injection to facilitate mocking in tests.
