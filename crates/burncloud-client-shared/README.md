# burncloud-client-shared

Shared components and styles for BurnCloud client applications.

This crate provides reusable UI components and style helpers used across BurnCloud desktop/web clients built with Dioxus.

- Components: layout, sidebar, title bar, placeholders
- Styles: global themes, common typography, spacing utilities

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
burncloud-client-shared = "0.1"
```

Then import components/styles as needed:

```rust
use burncloud_client_shared::*;
```

## License

MIT