# BurnCloud Project Development Constitution

**Version**: 1.10
**Effective Date**: 2025-12-07

## Preamble

BurnCloud is not just a tool; it is a platform dedicated to making local deployment and management of Large Language Models (LLMs) elegant, efficient, and controllable. This Constitution establishes the core values, architectural principles, and development standards for the project. All contributors (including human developers and AI assistants) must adhere to it.

---

## Chapter 1: Core Philosophy

### 1.1 Performance First
We choose Rust for our extreme pursuit of performance.
*   **Reject Bloat**: Be extremely restrained when introducing dependencies. If the standard library can solve it, do not introduce third-party libraries.
*   **Async First**: All I/O operations (network, file, database) must be asynchronous (Tokio-based).

### 1.2 Local First
*   User data belongs to the user. All configurations, databases (SQLite), and model files are stored locally on the user's machine.
*   Do not upload any telemetry or user privacy data unless explicitly authorized by the user.

### 1.3 Fluent Experience
*   Responsiveness must be fast, visual feedback smooth, and stuttering is rejected.

### 1.4 Internationalization Foundation
*   **i18n Native**: Our program is natively designed for global users. Hardcoding strings in any language within UI code is strictly prohibited.
*   **Bilingual Baseline**: **Chinese** and **English** are the baseline languages for the project. All features must fully support both languages upon release.
*   **Extensibility**: The architecture must reserve interfaces to easily support more languages in the future.

### 1.5 The Jobs Design Philosophy
*   **Zen & Focus**: The interface must be minimalist. Hide the plumbing (technical details) and highlight the magic (the model/intelligence).
*   **Delight**: Prioritize user delight over raw feature count. Use animations, whitespace, and visual hierarchy to create an emotional connection.
*   **"iTunes for Intelligence"**: The UI should feel like a premium macOS application—clean, intuitive, and polished. Simplicity is the ultimate sophistication.

### 1.6 The Expert Metaphor
*   **Experts, not Models**: Users don't care about `.bin` files. They care about capabilities. We present "Experts" (e.g., "Coding Wizard", "Creative Writer"), not just raw model names.
*   **Hide the Math**: Don't show quantization bits (INT4/FP16) by default. Use human-readable sliders: "Faster" <-> "Smarter".

### 1.7 BurnGrid Protocol
*   **Seamless Sharing**: Network features (reselling/channels) are branded as "**BurnGrid**". It should feel like AirPlay for Compute—toggle it on to share excess power, toggle off for privacy.
*   **Universal Memory**: We aim to provide a system-level vector memory that persists across different experts.

---

## Chapter 2: Architectural Principles

The project adopts a **Rust Workspace (Monorepo)** structure, following a strict layered architecture.

### 2.1 Modularization
Building monolithic applications is strictly prohibited. Functionality must be split into independent Crates:
*   **UI Layer (`crates/client`)**: Responsible only for rendering and interaction; contains no core business logic.
*   **Service Layer (`crates/service`)**: Pure Rust business logic, no UI dependencies.
*   **Data Layer (`crates/database`)**: Responsible for persistence, using SQLx.
*   **Router Layer (`crates/router`)**: Independent high-performance gateway component.
*   **Core Layer (`crates/core`)**: Shared underlying logic.

### 2.2 Unidirectional Dependency Flow
Dependencies must be clear and unidirectional:
`Client` -> `Service` -> `Database/Core`
Reverse dependencies or circular dependencies are prohibited.

---

## Chapter 3: The Router Doctrine

For the `crates/router` component, the following unshakable principles are established:

### 3.1 Passthrough Principle
**"Don't Touch the Body"** is the highest rule of the Router.
*   We are a **smart pipe**, not a processor.
*   **Strictly Prohibited**: Parsing, deserializing, or restructuring the Request/Response Body (unless forced by authentication mechanisms like AWS SigV4).
*   Keep Streaming response absolutely unobstructed to ensure zero-latency typewriter effects.

### 3.2 Minimal Protocol Adaptation
*   Do not attempt to unify JSON formats from various vendors.
*   The Router is responsible for **routing distribution**, **authentication replacement**, and **billing statistics**.
*   The client decides what format to use (OpenAI SDK uses OpenAI format, Claude SDK uses Claude format).

### 3.3 Independence and Lightweight
*   Complex authentication logic like AWS must be isolated in sub-Crates (e.g., `router-aws`).
*   Avoid introducing massive SDKs (like the full AWS SDK); prioritize using lightweight crypto libraries to hand-write signing logic to keep binary size small and compilation fast.

### 3.4 Protocol Adaptor Optionality
*   **Default Passthrough**: If the client uses a native protocol (e.g., Gemini SDK accessing Gemini), the Router **never** performs any format conversion.
*   **Explicit Trigger**: Only enable the protocol adaptor (Adaptor) for Request/Response conversion when the user explicitly requires it (e.g., specifying "Simulate OpenAI" via configuration or headers).

---

## Chapter 4: Engineering Standards

### 4.1 Commit Standards
Git commit messages must follow the **Emoji Prefix** format and clearly explain the changes in the description.

**Format**: `<Icon> <Type>: <Summary>`

| Icon | Type | Description |
| :--- | :--- | :--- |
| ✨ | `feat` | New Feature |
| 🐛 | `fix` | Bug Fix |
| 📚 | `docs` | Documentation Change |
| 🔨 | `refactor` | Code Refactoring (No feature change) |
| 🚀 | `perf` | Performance Optimization |
| 🧪 | `test` | Test Code Change |
| 🔧 | `chore` | Build process or auxiliary tool change |

**Example**:
*   `✨ feat: add aws sigv4 signing support`
*   `📚 docs: update CONSTITUTION.md`

### 4.2 Testing Standards
*   **Mandatory Unit Tests**: Mandatory unit tests must be written to verify core logic whenever an atomic development task is completed.
*   **Integration Tests**: Must be separated from production code.
    *   **Data Source Isolation**: API Keys or credentials required for testing must be injected via **environment variables** (e.g., `TEST_AWS_AK`) or read from a test-specific temporary database.
    *   **No Hardcoding**: Real API Keys, Access Key/Secret Keys **must absolutely not appear** in the source code (including files under `tests/`).
    *   **Mandatory Environment Variables**: All sensitive credentials in test cases must be obtained via `std::env::var`; hardcoding for convenience is strictly prohibited.
    *   **Sensitive Information Restoration**: If real Keys are temporarily written during local debugging or emergency fixes, **they must be restored to sanitized sample Keys (e.g., `YOUR_AK_HERE`) before committing code or finishing the task**.
    *   **No Sensitive Files**: Including JSON/YAML/ENV files storing real keys in the codebase is strictly prohibited.
    *   Tests must be idempotent and must not pollute the user's real database.
*   **E2E Testing Location & Structure**:
    *   **Mandatory Location**: All E2E (End-to-End) test files must be stored in the `/crates/tests` folder in the project root.
    *   **Path Correspondence**: The naming and directory structure of E2E test files must strictly correspond to the Router request path being tested.
        *   Example: An E2E test for route `POST /v1/chat/completions` should be located at `/crates/tests/v1/chat/completions_test.rs`.
        *   Example: An E2E test for route `GET /api/models` should be located at `/crates/tests/api/models_test.rs`.
*   **Automated E2E Execution**:
    *   **Self-Bootstrapping**: E2E test code must be "self-bootstrapping," meaning it automatically starts the service under test (Router/Server) and waits for it to be ready. Relying on externally pre-started processes is strictly prohibited.
    *   **Fully Automated Loop**: Whether executed by a developer, CI, or AI Agent, tests must complete the full "Start Service -> Run Test -> Stop Service" loop via a single command.
*   **Test Before Commit**: Ensure `cargo test` passes before marking a task as complete.

### 4.3 Error Handling
*   Use `anyhow` for top-level errors and `thiserror` for library-level errors.
*   Using `unwrap()` is strictly prohibited unless in Test code or you are 100% sure it will not Panic (and comment the reason).

### 4.4 Atomic Development
*   **Minimum Unit**: Each development must be granular to the "Minimum Viable Unit" (e.g., only support DeepSeek's AuthType, not all domestic models at once).
*   **Development Loop**: Must follow the complete loop of `Plan` -> `Code` -> `Test` -> `Commit`. Starting the next unit before the previous one passes tests is strictly prohibited.
*   **Step-by-Step Commit**: Avoid "Big Bang" code commits.

### 4.5 Zero Warning Tolerance
*   **Rustc/Clippy Clean**: Code must pass `cargo check` and `cargo clippy` without generating any Warnings.
*   **Clean Unused Code**: Retaining unused imports, variables, or dead code is strictly prohibited.
*   **Naming Conventions**: Strictly adhere to Rust's naming conventions (e.g., snake_case).

### 4.6 Code Layout
*   **No Useless Newlines**: Iron Rule—Useless newlines in code are not allowed. Code should remain compact, keeping necessary empty lines only between logical blocks.

### 4.7 Dependency Management
*   **Version Unification**: Version numbers of all dependency packages must be uniformly declared in `[workspace.dependencies]` in the root `Cargo.toml`.
*   **No Scattering**: Sub-Crates' `Cargo.toml` must use `workspace = true` to reference dependencies; hardcoding version numbers in sub-Crates is strictly prohibited.

### 4.8 Marketing First
*   **Copywriting First**: When updating `README.md`, prioritize marketing copy, clearly highlighting core functions (Features) and user value (Benefits).
*   **Visual Appeal**: Make good use of badges, emojis, and clear layout to ensure documentation is attractive to developers and users.

---

## Chapter 5: Security Protocols

### 5.1 Credential Management
*   **Zero Trust**: It is recommended to store all API Keys encrypted in the database (future plan).
*   **Zero Leakage**: Real Access Keys or Secret Keys are strictly prohibited in Git history. If accidentally committed, the key must be immediately revoked and Git history rewritten.

### 5.2 Authentication
*   Router must verify the user's Bearer Token before forwarding requests.
*   Exposed ports default to binding `127.0.0.1` unless the user explicitly configures `0.0.0.0`.

---

## Chapter 6: AI Agent Protocol

All AI agents assisting in development must adhere to the following reporting process when completing tasks:

### 6.1 Reporting Language
*   Regardless of the language used by the user to ask questions, the AI agent must use **Chinese** when **summarizing update content**.
*   This helps maintain consistency in project documentation and communication (the core project language is Chinese).

### 6.2 Git Message Generation
*   **Write to File**: In every response involving code changes, the AI agent must **overwrite** the generated Git Commit Message to the `MESSAGE.md` file in the project root.
*   **No Chat Output**: The Git Commit Message will **no longer** be displayed directly in the chat window.
*   **Formatting Standards**:
    *   Language: **English**.
    *   Structure: Strictly adhere to the Emoji format in **4.1 Commit Standards**.
    *   Content: Must include a **Simple functional update description**, clearly and accurately describing the changes.
    *   **Prohibit Markdown**: The generated Commit Message is strictly prohibited from using Markdown code block markers (such as ```), it must be plain text for easy copying.

### 6.3 Strictly No Auto-Commit
*   **Iron Rule**: AI agents/programming tools are **strictly prohibited** from directly executing the `git commit` command.
*   **Generation Only**: The AI agent can only generate the commit message (via `MESSAGE.md`) and **must rely on the human user** to personally confirm and execute the commit operation.

---

## Appendix: Directory Structure Mapping

```
burncloud/
├── crates/
│   ├── client/          # Dioxus GUI (Fluent Design)
│   ├── router/          # LLM Passthrough Gateway (Axum)
│   │   └── crates/router-aws # AWS SigV4 Logic
│   ├── service/         # Business Logic
│   ├── database/        # SQLx/SQLite Logic
│   └── common/          # Shared Types
├── tests/               # End-to-End Integration Tests
└── src/                 # Application Entry (main.rs)
```

---
*This Constitution is established by the BurnCloud Architect, and all subsequent development must be based on this.*