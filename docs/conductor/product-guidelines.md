# Product Guidelines: BurnCloud

## 1. Visual Identity & UX
*   **Aesthetic Philosophy:** **Steve Jobs' Apple Aesthetic**.
    *   Focus on simplicity, elegance, and extreme attention to detail.
    *   Clean typography, purposeful whitespace, and subtle, meaningful animations.
    *   The interface should feel "inevitable" and intuitive, hiding complexity behind a beautiful facade.
*   **Tone of Voice:** **Friendly & Onboarding**.
    *   While the technology is complex, the user experience should be welcoming.
    *   Error messages should be human-readable and offer solutions, not just codes.
    *   Documentation should guide users from zero to hero with patience.

## 2. Architectural Principles
*   **Structure:** **Modular Monorepo**.
    *   The workspace is divided into distinct crates (`router`, `client`, `service`, `database`, `core`) to enforce separation of concerns.
    *   This supports parallel compilation and allows the Core/Router to be potentially reused in other contexts.

## 3. Coding Standards & Constraints
*   **Safety First:** **Zero Unsafe** policy. `unsafe` blocks are forbidden unless wrapping essential FFI, and must be documented with rationale.
*   **Concurrency:** **Async/Tokio** is the standard for all I/O. Blocking operations in async contexts is strictly prohibited.
*   **Type Safety:** **Strict Typing** is enforced. Use the "Newtype" pattern (e.g., `struct UserId(String)`) to prevent primitive obsession and logic errors.
*   **The Passthrough Directive:**
    *   The Router's primary mandate is speed.
    *   It **MUST NOT** deserialize/parse the JSON body of model requests (except for specific legacy adaptors).
    *   It operates as a smart pipe, streaming bytes directly to ensure near-zero latency.
