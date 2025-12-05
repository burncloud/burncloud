# BurnCloud - Gemini Context

> **IMPORTANT:** Before modifying code, please read `docs/CONSTITUTION.md` for architectural principles and coding standards.

This document provides context for the BurnCloud project to assist Gemini in understanding the codebase and development environment.

## 1. Project Overview

**BurnCloud (奔云)** is a local AI model deployment and gateway platform built with **Rust**. It combines a modern Dioxus-based desktop GUI for model management with a high-performance **LLM API Router**.

*   **Core Identity:** A unified platform to Manage, Deploy, and Route AI Models.
*   **Operating System:** Windows 10/11 (Primary target with Fluent Design), Cross-platform core.
*   **Language:** Codebase and UI interactions are primarily in **Chinese (中文)**.

## 2. Technical Architecture

The project is a **Rust Monorepo** organized as a Cargo workspace, adhering to a strict modular architecture defined in `docs/CONSTITUTION.md`.

### Tech Stack
*   **GUI Framework:** [Dioxus](https://dioxuslabs.com/) (Desktop)
*   **Web Framework:** [Axum](https://github.com/tokio-rs/axum) (Router/Gateway)
*   **Runtime:** Tokio (Async I/O)
*   **Database:** SQLite (via `sqlx`)
*   **HTTP Client:** reqwest (with streaming support)

### Workspace Structure (`crates/`)

*   **Router (High-Performance Gateway):** `crates/router`
    *   **Function:** A "Passthrough" reverse proxy for OpenAI/Claude/Bedrock APIs.
    *   **Logic:** Does *not* parse bodies ("Don't Touch the Body" principle) to ensure zero-latency streaming.
    *   **Components:**
        *   `crates/router-aws`: Isolated AWS SigV4 signing logic (Manual implementation, no full SDK).
    *   **Database:** `crates/database/crates/database-router` (Manages Upstreams and Tokens).
*   **Client (GUI):** `crates/client`
    *   Uses Dioxus Router for navigation.
    *   Feature crates: `client-dashboard`, `client-models`, `client-deploy`, `client-settings`.
    *   `client-shared`: Implements Windows 11 Fluent Design via custom CSS (`styles.rs`).
*   **Services (Business Logic):** `crates/service`
    *   `service-models`: HuggingFace API integration (Region-aware).
    *   `service-ip`: Geolocation detection (CN vs WORLD).
*   **Database (Persistence):** `crates/database`
    *   Centralized `sqlx` SQLite connection management.
    *   Modularized schemas: `database-models`, `database-setting`, `database-router`.
*   **Core & Common:**
    *   `crates/core`: Core application logic.
    *   `crates/common`: Shared utilities and types.
    *   `crates/cli`: Command-line interface.

## 3. Development & Usage

### Key Commands

| Action | Command | Description |
| :--- | :--- | :--- |
| **Run Router** | `cargo run -- router` | Starts the LLM Gateway (Port 3000). |
| **Run GUI** | `cargo run -- client` | Launches the desktop application. |
| **Test Router** | `cargo test -p burncloud-router --test integration_test` | Runs End-to-End router tests (Requires env vars). |
| **Test Unit** | `cargo test -p burncloud-router-aws` | Runs unit tests for AWS signing logic. |
| **Build All** | `cargo build` | Builds the entire workspace. |

### Design System (Fluent Design)
The Client UI strictly implements Windows 11 Fluent Design:
*   **Visuals:** Mica effects, rounded corners, depth shadows.
*   **Code:** `crates/client-shared/src/styles.rs`.

## 4. Key Implementation Principles

### The Router Doctrine ("Passthrough")
*   **Concept:** The router acts as a smart pipe. It authenticates users, routes based on path prefixes (e.g., `/v1/messages` -> Anthropic), and injects upstream API keys.
*   **Streaming:** Request/Response bodies are streamed directly without buffering (except for AWS Bedrock which requires buffering for signature calculation).
*   **AWS Support:** Uses a custom, lightweight implementation of SigV4 (`router-aws`) to sign requests for Bedrock, avoiding heavy AWS SDK dependencies.

### Regional Routing
*   **HuggingFace:** Auto-switches to `hf-mirror.com` if the user is detected in China (`service-ip`).
*   **AWS:** Router logic supports regional endpoints (e.g., `us-east-1`, `us-west-2`).

## 5. Current Status (as of Dec 2025)

*   ✅ **Router Core:** Fully functional Axum-based passthrough gateway.
*   ✅ **AWS Bedrock Support:** SigV4 signing implemented and verified via integration tests.
*   ✅ **Database Integration:** Router configuration (Upstreams/Tokens) persisted in SQLite.
*   🚧 **GUI Integration:** Dashboard currently does not yet visualize Router statistics.
*   🚧 **Model Downloads:** Active development on Aria2 integration for model files.

## 6. Documentation References
*   `docs/CONSTITUTION.md`: The supreme development guidelines.
*   `docs/TASK.md`: Current development roadmap and task status.
*   `docs/PLAN.md`: Initial design document for the Router.