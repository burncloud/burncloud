# BurnCloud Rust UI

Leptos CSR/WebAssembly migration of the BurnCloud Buyer Overview.

## Prerequisites

```powershell
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
```

## Run

```powershell
trunk serve
```

Open <http://127.0.0.1:8080/>.

## Verify

```powershell
cargo fmt --all -- --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
cargo test
trunk build --release
```
