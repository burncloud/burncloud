# BurnCloud - Gemini Context

This document provides context for the BurnCloud project to assist Gemini in understanding the codebase and development environment.

## 1. Project Overview

**BurnCloud (奔云)** is a local AI model deployment platform built with **Rust** and **Dioxus**. It focuses on providing a modern, user-friendly desktop GUI for managing and deploying Large Language Models (LLMs) like Qwen, DeepSeek, and Llama.

*   **Target OS:** Windows 10/11 (Primary), with Fluent Design aesthetics.
*   **Core Functionality:** Model management, one-click deployment, system monitoring, and API management.
*   **Language:** The codebase and UI primarily use **Chinese (中文)**.

## 2. Technical Architecture

The project is a **Rust Monorepo** organized as a Cargo workspace.

### Tech Stack
*   **Language:** Rust (Edition 2021)
*   **Frontend/GUI:** [Dioxus](https://dioxuslabs.com/) (Desktop mode)
*   **Async Runtime:** Tokio
*   **Database:** SQLite (via `sqlx`)
*   **HTTP:** reqwest
*   **CLI:** clap

### Workspace Structure (`crates/`)

The workspace is divided into several layers:

*   **Client (GUI):** `crates/client`
    *   Uses Dioxus Router for navigation.
    *   Modularized into feature crates: `client-dashboard`, `client-models`, `client-deploy`, `client-monitor`, `client-settings`.
    *   `client-shared` contains common components and the Fluent Design style system (`styles.rs`).
*   **Server:** `crates/server`
    *   Backend server component (likely for API handling or local serving).
*   **Services (Business Logic):** `crates/service`
    *   `service-models`: HuggingFace API integration, model metadata.
    *   `service-ip`: Geolocation (CN vs WORLD) for mirror selection.
    *   `service-monitor`: System resource monitoring.
*   **Database (Persistence):** `crates/database`
    *   `database-models`: Stores model info.
    *   `database-setting`: Key-value store for app config.
    *   `database-download`: Tracks download state.
*   **Core & Common:**
    *   `crates/core`: Core business logic.
    *   `crates/common`: Shared types, errors, and utilities.
    *   `crates/cli`: Command-line interface.

## 3. Development & Usage

### Key Commands

| Action | Command | Description |
| :--- | :--- | :--- |
| **Build All** | `cargo build` | Builds the entire workspace. |
| **Run GUI** | `cargo run` or `cargo run -- client` | Launches the desktop application. |
| **Run Server** | `cargo run -- server` | Starts the backend server. |
| **Run CLI** | `cargo run -- code` | Runs the CLI tool. |
| **Test All** | `cargo test` | Runs unit and integration tests. |
| **Test Crate** | `cargo test -p burncloud-database` | Tests a specific crate. |

### Design System (Fluent Design)
The UI implements Windows 11 Fluent Design principles manually via CSS in `crates/client-shared/src/styles.rs`:
*   **Colors:** Defined Accent and Neutral palettes.
*   **Layout:** CSS Grid with `auto-fit` and `minmax` for responsiveness.
*   **Effects:** Mica material, rounded corners, and shadows.

## 4. Key Implementation Details

*   **Regional Routing:** The app detects if the user is in China (`service-ip`) and switches to `hf-mirror.com` for HuggingFace downloads; otherwise, it uses `huggingface.co`.
*   **Database:** SQLite database is stored in the user's data directory (managed by `dirs` crate). Boolean fields are often stored as integers (0/1).
*   **Async Recursion:** Recursive async functions (e.g., traversing file trees) use `Box::pin`.

## 5. File Locations
*   **Entry Point:** `src/main.rs` (Routes to Client, Server, or CLI based on args).
*   **Root Config:** `Cargo.toml` (Workspace definition).
*   **Styles:** `crates/client/crates/client-shared/src/styles.rs`.
*   **Route Definitions:** `crates/client/src/app.rs`.

## 6. Current Status
*   Active development on Model Download functionality.
*   Recent features: Location detection, Settings database, HuggingFace integration.
