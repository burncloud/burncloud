# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

BurnCloud (奔云) is a local AI model deployment platform built with Rust and Dioxus. It provides a modern desktop GUI for managing and deploying large language models (LLMs) like Qwen, DeepSeek, and Llama with a Windows 11 Fluent Design-inspired interface.

## Build and Run Commands

### Building
```bash
# Build the entire workspace
cargo build

# Build in release mode
cargo build --release

# Build a specific crate
cargo build -p burncloud-client
```

### Running
```bash
# Run GUI client (default on Windows)
cargo run

# Run GUI client explicitly
cargo run -- client

# Run server component
cargo run -- server

# Run CLI
cargo run -- code

# Check for updates
cargo run -- update --check-only

# Execute update
cargo run -- update
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p burncloud-database

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

## Architecture

### Monorepo Structure

This is a Cargo workspace with 23+ crates organized by functionality:

**Client Crates** (Dioxus-based GUI):
- `burncloud-client` - Main GUI application entry point, routing, and layout
- `burncloud-client-shared` - Shared components and Fluent Design styles
- `burncloud-client-dashboard` - System status and monitoring overview
- `burncloud-client-models` - Model management and repository browsing
- `burncloud-client-deploy` - Deployment configuration interface
- `burncloud-client-monitor` - Real-time resource monitoring
- `burncloud-client-settings` - System settings and preferences
- `burncloud-client-api` - API testing and documentation
- `burncloud-client-tray` - System tray integration (Windows)

**Server/Service Crates**:
- `burncloud-server` - Backend server component
- `burncloud-service-models` - Model information services with HuggingFace API integration
- `burncloud-service-monitor` - System monitoring services
- `burncloud-service-ip` - Network location detection (CN vs WORLD)
- `burncloud-service-setting` - Configuration management service

**Database Crates**:
- `burncloud-database` - Core database abstractions and SQLx integration
- `burncloud-database-models` - Model metadata storage (ModelInfo struct)
- `burncloud-database-download` - Download tracking and management
- `burncloud-database-setting` - Key-value configuration storage

**Download Crates**:
- `burncloud-download` - Download orchestration
- `burncloud-download-aria2` - Aria2 integration for efficient downloads

**Core Crates**:
- `burncloud-core` - Core business logic
- `burncloud-common` - Shared utilities and types
- `burncloud-cli` - Command-line interface with clap
- `burncloud-code` - Code execution component
- `burncloud-auto-update` - GitHub-based auto-update functionality

### Nested Crate Structure

The workspace uses a nested crate organization pattern:

```
crates/
├── service/
│   └── crates/
│       ├── service-ip/          # Location detection
│       ├── service-models/      # Model management
│       ├── service-monitor/     # System monitoring
│       └── service-setting/     # Configuration
├── database/
│   └── crates/
│       ├── database-models/     # Model metadata
│       ├── database-download/   # Download tracking
│       └── database-setting/    # Settings storage
├── client/
│   └── crates/
│       ├── client-models/       # Model UI
│       ├── client-dashboard/    # Dashboard UI
│       └── ...
└── download/
    └── crates/
        └── download-aria2/      # Aria2 backend
```

**Location Detection**:
- `burncloud-service-ip` - Network location detection using ip-api.com with ipinfo.io fallback
- Returns "CN" for China, "WORLD" for other regions
- Cached in `burncloud-database-setting` with name="location"

### Application Entry Points

The main binary (`src/main.rs`) routes to three modes:
1. **GUI Mode** (`client`): Launches Dioxus desktop app with system tray
2. **Server Mode** (`server`): Starts backend server
3. **CLI Mode** (default for other args): Handles commands via `burncloud-cli`

### GUI Architecture (Dioxus)

The client uses Dioxus Router with a Layout component wrapping all pages:

**Route Structure**:
- `/` → Dashboard (system overview)
- `/models` → ModelManagement (browse, download, manage models)
- `/deploy` → DeployConfig (resource allocation, quantization settings)
- `/monitor` → ServiceMonitor (real-time logs and performance)
- `/api` → ApiManagement (API testing and docs)
- `/settings` → SystemSettings (theme, language, security)

**Styling System**:
Located in `crates/client-shared/src/styles.rs`, implements Fluent Design with:
- CSS custom properties for colors (Accent/Neutral palettes)
- Spacing system (xs, sm, md, lg, xl, xxl, xxxl)
- Border radius levels (small, medium, large, xlarge)
- Shadow system (card, flyout, dialog)
- Responsive Grid layouts with `auto-fit` and `minmax()`

### Database Architecture

Uses SQLx with SQLite for persistent storage:
- Database location: Default to user data directory via `dirs` crate
- Schema defined in migrations
- Three main database modules:
  - **Models**: Stores AI model metadata from HuggingFace (ModelInfo struct with JSON fields)
  - **Settings**: Key-value configuration store (name as primary key)
  - **Downloads**: Download progress and state tracking
- Boolean fields stored as integers (0/1) with custom serde converters

### HuggingFace Integration

**Service Models** (`burncloud-service-models`):
- `get_huggingface_host()`: Returns region-specific HuggingFace host
  - CN: `https://hf-mirror.com/` (mirror site)
  - WORLD: `https://huggingface.co/` (official site)
  - Cached in settings with name="huggingface"
- `fetch_from_huggingface()`: Fetches model list from HuggingFace API
- `get_model_files(model_id)`: Recursively fetches all files for a model
  - Returns `Vec<Vec<String>>` with format: `[type, oid, size, path]`
  - Automatically traverses directory structures

**Data Flow**:
1. Location detected by `service-ip` → cached in `database-setting`
2. HuggingFace host determined by location → cached in `database-setting`
3. API calls use cached host for consistent regional routing

### Workspace Dependencies

All crates use workspace-level dependency management defined in root `Cargo.toml`:
- UI: `dioxus`, `dioxus-router`, `dioxus-desktop`
- Async: `tokio` with full features
- Database: `sqlx` with SQLite and async runtime
- HTTP: `reqwest` with JSON and streaming
- CLI: `clap` with derive features
- Error handling: `anyhow`, `thiserror`
- Serialization: `serde`, `serde_json`

## Development Practices

### Language and Communication
- This is a Chinese-language project. Code comments, commit messages, and documentation are primarily in Chinese (中文)
- UI text is in Chinese: "大模型本地部署平台" (Large Model Local Deployment Platform)

### Code Style
- Follow standard Rust conventions
- Use workspace dependencies (`.workspace = true`) in crate Cargo.tomls
- Keep crates focused: separate client pages, database concerns, and services

### Windows-Specific Features
- Primary target is Windows 10/11 with Fluent Design
- Uses `winapi` for Windows-specific system calls
- Decorations disabled on main window for custom title bar
- System tray integration via `systray` crate

### Testing
Tests are primarily in the `database` crate:
- Unit tests: `crates/database/tests/database_unit_tests.rs`
- Integration tests: `crates/database/tests/integration_tests.rs`
- Performance tests: `crates/database/tests/performance_tests.rs`
- Cross-platform tests: `crates/database/tests/cross_platform_tests.rs`
- Error handling tests: `crates/database/tests/error_handling_tests.rs`

## Key Design Patterns

### Component-Based Architecture
Each client page is a self-contained crate with its own component structure, reducing coupling and enabling independent development.

### Database Abstraction
The `burncloud-database` crate provides generic database traits, while specific implementations like `burncloud-database-models` build domain-specific functionality on top.

### Service Layer

Service crates (`burncloud-service-*`) provide business logic separate from both UI and persistence layers:

**Service Models** (`burncloud-service-models`):
- CRUD operations for model metadata
- HuggingFace API integration with regional routing
- File tree traversal for model repositories
- Caches HuggingFace host in settings

**Service IP** (`burncloud-service-ip`):
- Detects user's network location (CN vs WORLD)
- Primary API: http://ip-api.com/json/
- Fallback API: https://ipinfo.io/
- Caches result in settings with name="location"

**Service Setting** (`burncloud-service-setting`):
- Thin wrapper over `database-setting`
- Provides async CRUD for configuration values
- Common settings: "location", "huggingface", "data_dir"

### Modular Downloads
Download functionality is abstracted with a pluggable backend system (currently Aria2), allowing for alternative download engines.

## Running Examples

Service crates include example programs demonstrating functionality:

```bash
# Test HuggingFace host detection and caching
cargo run -p burncloud-service-models --example test_host

# Fetch model file list recursively
cargo run -p burncloud-service-models --example get_files

# Test location detection
cargo run -p burncloud-service-ip --example get_location

# Check all settings
cargo run -p burncloud-service-models --example check_settings
```

## Important Implementation Details

### Async Recursion
When implementing recursive async functions, use `Box::pin` to avoid infinite-sized futures:

```rust
fn recursive_async<'a>(args: &'a Type) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        // recursive call works here
        recursive_async(new_args).await?;
        Ok(())
    })
}
```

### Regional Routing Pattern
The codebase uses a consistent pattern for China vs international routing:
1. Check settings cache first (`service-setting`)
2. If not cached, detect location (`service-ip`)
3. Save to cache for future use
4. Return region-specific value (URLs, API endpoints, etc.)

This pattern minimizes API calls and ensures consistent behavior across sessions.

## Current Development Focus

According to `docs/TODO.md`, recent completions and active work:

**Recently Completed**:
- ✅ HuggingFace API integration with regional routing
- ✅ Location detection with caching
- ✅ Settings database and service layer
- ✅ Model search dialog in client-models
- ✅ Recursive file tree traversal for model repositories

**Active Development**:
- Model download functionality in `client-models`
- Integration with `download-aria2` backend
- Data directory configuration management
