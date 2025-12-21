# Product Guide: BurnCloud

## 1. Vision & Goals
**BurnCloud** is a high-performance, Rust-native LLM Aggregation Gateway and Management Platform.
**Goal:** To provide a resource-efficient, secure, and unified access layer for LLMs, surpassing existing solutions like One API in performance and reliability.
**Core Value:**
*   **Performance First:** Built on Rust/Axum for minimal memory footprint and high concurrency.
*   **Local Sovereignty:** Strong focus on local data ownership and privacy.
*   **Reliability:** "Don't Touch the Body" architecture ensuring zero-latency passthrough and robust failover.

## 2. Target Audience
*   **Individual Developers:** For unified API access and local tool integration.
*   **Enterprises & Teams:** For centralized management, quota enforcement, and multi-user access.
*   **API Resellers:** For aggregating and distributing model access with billing capabilities.

## 3. Key Features
*   **Unified API Gateway:** seamless routing to OpenAI, Anthropic, Gemini, and other providers via a standard interface.
*   **User Management & Billing:** Comprehensive system for managing quotas, tokens, and multi-tenant billing.
*   **Desktop GUI:** A Windows-fluent local client (Dioxus) for visual management of routing, logs, and system settings.
*   **Smart Routing:** Advanced load balancing (weighted), automatic failover, and regional routing optimizations.

## 4. Technical Architecture Strategy
*   **Traffic Management:** A **Hybrid Approach** combining weighted distribution with circuit breakers and automatic retries to ensure 99.9% availability.
*   **Deployment:**
    *   **Single Binary:** Zero-dependency executable.
    *   **Data Persistence:** Local SQLite database for configuration and logs.
    *   **Cross-Platform:** Native support for Windows, Linux, and macOS.

## 5. Success Metrics
*   **System:** < 50MB idle memory usage, zero GC pauses.
*   **Reliability:** Zero regression in E2E tests against real APIs.
*   **User Experience:** "Fluent" UI responsiveness and strictly typed API interactions.
