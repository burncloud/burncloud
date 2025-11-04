# BurnCloud Client Tray

A simple system tray application for BurnCloud client.

## Features

- System tray integration
- Launch web interface (http://127.0.0.1:8080)
- Clean exit functionality
- Cross-platform support

## Usage

### As a library

```rust
use burncloud_client_tray::start_tray;

fn main() {
    start_tray().unwrap();
}
```

### As a binary

```bash
cargo run
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
burncloud-client-tray = "0.1.0"
```

## Requirements

- The application expects an icon file at `./res/burncloud.ico`
- BurnCloud web service should be running on http://127.0.0.1:8080

## License

Licensed under either of

* Apache License, Version 2.0
* MIT license

at your option.