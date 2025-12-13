<div align="center">

# BurnCloud

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue?style=for-the-badge)
![Tests](https://img.shields.io/badge/Tests-Passing-success?style=for-the-badge)

**The Next-Gen High-Performance AI Gateway & Aggregator**

[Feature Requests](https://github.com/burncloud/burncloud/issues) · [Roadmap](docs/ARCHITECTURE_EVOLUTION.md) · [Documentation](docs/)

[English](README.md) | [简体中文](docs/README_CN.md)

</div>

---

## 💡 What is BurnCloud?

BurnCloud is a **Rust-native** LLM Aggregation Gateway and Management Platform.
It aims to benchmark against and surpass **One API (New API)**, providing individual developers, teams, and enterprises with a **high-performance, resource-efficient, secure, and controllable** unified LLM access layer.

**We are not just reinventing the wheel; we are upgrading the engine.**
If you are tired of the high memory consumption, GC pauses, or complex deployment dependencies of existing gateways, BurnCloud is your best choice.

## ✨ Why BurnCloud? (Core Values)

### 🚀 1. Performance First
*   **Powered by Rust**: Built on `Axum` and `Tokio`, offering astonishing concurrency handling capabilities and extremely low memory footprint (MB level vs GB level).
*   **Zero-Overhead Passthrough**: Featuring a unique "Don't Touch the Body" routing mode. In scenarios without protocol conversion, it achieves byte-level zero-copy forwarding with near-zero latency.
*   **Single Binary**: No Runtime dependencies (No Python, No Node.js, No Java). One file is a complete platform.

### 🔌 2. Universal Aggregation
*   **All to OpenAI**: Unifies protocols from Anthropic (Claude), Google (Gemini), Azure, Alibaba Qwen, and other mainstream models into standard **OpenAI format**.
*   **Write Once, Run Anywhere**: Your LangChain, AutoGPT, or any existing application can seamlessly switch underlying models just by changing the Base URL.

### ⚖️ 3. Enterprise Governance
*   **Smart Load Balancing**: Supports Multi-Channel Round-Robin, Weighted Distribution, and Automatic Failover. If one `gpt-4` goes down, thousands of `gpt-4` stand up.
*   **Precise Billing**: Supports precise token-based billing, custom Model Ratios, and User Group Ratios.
*   **Multi-Tenant Management**: Comprehensive redemption codes, quota management, and invitation mechanisms.

### 🛡️ 4. Rock-Solid Reliability
*   **Real-World E2E Testing**: We have abandoned fake Mock data. BurnCloud's CI/CD pipeline validates end-to-end against **real OpenAI/Gemini APIs**, ensuring core forwarding logic remains robust in real network environments.
*   **Browser-Driven Verification**: Built-in automated UI tests based on **Headless Chrome** ensure the rendering link from Backend API to Frontend Dioxus LiveView is unobstructed.
*   **Zero-Regression Promise**: Strict **"API-Path Matching"** testing strategy ensures every Commit passes rigorous automated auditing.

### 🎨 5. Fluent Experience
*   **More Than API**: Built-in local management client developed with **Dioxus**, featuring **Windows 11 Fluent Design**.
*   **Visual Monitoring**: View real-time TPS, RPM, and token consumption trends, saying goodbye to boring log files.

---

## 🏗️ Architecture

BurnCloud adopts a strict four-layer architecture to ensure high cohesion and low coupling:

*   **Gateway Layer (`crates/router`)**: Data plane. Handles high-concurrency traffic, authentication, rate limiting, and protocol conversion.
*   **Control Layer (`crates/server`)**: Control plane. Provides RESTful APIs for UI calls, managing configuration and state.
*   **Service Layer (`crates/service`)**: Business logic. Encapsulates core logic like billing, monitoring, and channel speed testing.
*   **Data Layer (`crates/database`)**: Data persistence. Based on SQLx + SQLite/PostgreSQL, with future Redis cache support.

> See: [Architecture Evolution](docs/ARCHITECTURE_EVOLUTION.md)

---

## 🛠️ Getting Started

### Requirements
*   Rust 1.75+
*   Windows 10/11, Linux, or macOS

### Development Run

```bash
# 1. Clone repository
git clone https://github.com/burncloud/burncloud.git
cd burncloud

# 2. Configure (Optional)
cp .env.example .env
# Edit .env and fill in TEST_OPENAI_KEY to enable full E2E tests

# 3. Run (Auto-compiles Server and Client)
cargo run
```

### Run Tests (Quality Assurance)

Experience the industrial-grade testing process:

```bash
# Run all API integration tests
cargo test -p burncloud-tests --test api_tests

# Run UI automation tests (Requires Chrome)
cargo test -p burncloud-tests --test ui_tests
```

---

## 🗺️ Roadmap

- [x] **v0.1**: Basic routing & AWS SigV4 signing support (Completed)
- [x] **v0.2**: Database integration, Basic Auth & **New API Core Replication** (Completed)
    - [x] Ability Smart Routing
    - [x] Channel Management API
    - [x] Async Billing & Logging
- [x] **v0.3**: Unified Protocol Adaptors (OpenAI/Gemini/Claude) & E2E Test Suite (Completed)
- [ ] **v0.4**: Smart Load Balancing & Failover (In Progress)
- [ ] **v0.5**: Web Console Frontend Polish
- [ ] **v1.0**: Official Release, Redis Cache Integration

---

## 🤝 Contributing

Contributions of any kind are welcome! Please read our **[Development Constitution](docs/CONSTITUTION.md)** before submitting code.

## 📄 License

MIT License © 2025 BurnCloud Team
