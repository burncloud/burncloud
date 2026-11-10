# Agent Constraints (Ver 1.1)

This file defines the mandatory constraints for project development, applicable to all modules and code files. Only MUST and MUST NOT constraints are stated here.

---

## 1. Coding and Organization

- MUST follow Rust community conventions for directory structure, file naming, code structure, and function/variable naming.

## 2. Comment Rules

- MUST write comments in Chinese.
- MUST use `///` Markdown doc-comments for public APIs, covering purpose, examples, and notes.
- MUST use `//` for internal implementation, explaining "why" rather than "what".

## 3. Visibility Constraints

- MUST default the visibility of all newly added functions, methods, and struct members to private. MUST make them public only when explicitly marked as public in interface definitions, API documentation, or a public manifest.
- MUST change the visibility of existing functions, methods, or struct members only when explicitly declared.

## 4. Parameter Validation

- MUST perform black-box parameter safety validation for public interfaces.
- MUST perform white-box parameter safety validation for public functions.
- MUST NOT perform parameter safety validation for private functions.
- MUST prioritize simplicity.

## 5. No Over-Engineering

- MUST follow Occam's Razor principle: *"Entities must not be multiplied beyond necessity."*
- MUST NOT add architecture in advance for "future possibilities".
- MUST NOT introduce excessive abstraction layers.
- MUST NOT over-decouple.
- MUST NOT create extension points far exceeding real usage.
- MUST NOT let common/shared libraries grow out of control.
- MUST NOT over-inflate tests and CI.
- MUST NOT add patterns for the sake of adding patterns.

## 6. Error Handling

- MUST use `Result` for recoverable errors and `panic` for unrecoverable errors, following Rust community conventions.
- MUST NOT silently swallow errors (e.g., `let _` on errors). If an error must be ignored, MUST annotate the reason.

## 7. Safety Red Lines

- MUST NOT use `unsafe`. If truly required, MUST explicitly evaluate and document the reason.
- MUST NOT use `unwrap()`/`expect()` on external input.

## 8. Performance and Resources

- MUST avoid unnecessary heap allocations or clones on hot paths.
- MUST prefer borrows and references over ownership transfer, following the borrow checker semantics.

## 9. Testing

- MUST provide unit tests for every public API.
- MUST provide doc-tests (`/// # Examples`) for critical modules.
- MUST add regression tests for every fixed bug.
- MUST isolate test code from source code: MUST NOT include both production source code and test code in the same file.

## 10. Commit and Collaboration

- MUST follow Conventional Commits (`feat`/`fix`/`refactor`/`docs`, etc.).
- MUST run `cargo fmt`, `cargo clippy`, and `cargo test` before and after commit.

## 11. Documentation Sync

- MUST synchronously update doc-comments and README when modifying public APIs, keeping the "code as documentation" style.
