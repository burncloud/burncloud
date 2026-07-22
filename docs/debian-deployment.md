# Debian Deployment

BurnCloud runs as a headless LiveView server on Debian. The web dashboard and
gateway are served by the same `burncloud server` process; no GTK or WebKit
packages are required.

## Install a release binary

Use the `x86_64-unknown-linux-gnu` archive from the GitHub release, then
install the binary and service definition:

```bash
tar -xzf burncloud-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
cd burncloud-vX.Y.Z-x86_64-unknown-linux-gnu
sudo useradd --system --home /var/lib/burncloud --create-home --shell /usr/sbin/nologin burncloud
sudo install -m 0755 burncloud /usr/local/bin/burncloud
sudo install -D -m 0644 burncloud.service /etc/systemd/system/burncloud.service
sudo install -d -m 0750 -o root -g burncloud /etc/burncloud
```

Create `/etc/burncloud/burncloud.env` with restrictive permissions:

```bash
sudo install -m 0600 -o root -g burncloud /dev/null /etc/burncloud/burncloud.env
sudoedit /etc/burncloud/burncloud.env
```

Set at least the following values. `JWT_SECRET` must be stable across restarts.
`MASTER_KEY` is optional on the first start, but should be copied from
`/var/lib/burncloud/.env` and then managed here before a later deployment.

```ini
JWT_SECRET=replace-with-a-random-secret
MASTER_KEY=64-hex-character-encryption-key
BURNCLOUD_INTERNAL_SECRET=replace-with-a-second-random-secret
RUST_LOG=info
# BURNCLOUD_DATABASE_URL=postgres://user:password@host:5432/burncloud
```

Start and inspect the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now burncloud
sudo systemctl status burncloud
curl http://127.0.0.1:3000/health
```

The default SQLite database is stored at `/var/lib/burncloud/.burncloud/data.db`.
Use PostgreSQL via `BURNCLOUD_DATABASE_URL` when multiple application instances
need to write concurrently.

Official release archives target Debian x86_64. The Rust source build also
supports Debian arm64, but arm64 release archives are not published yet.

## Build from source

Debian 12/Bookworm packages for a source build:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev ca-certificates curl
curl https://sh.rustup.rs -sSf | sh -s -- -y
. "$HOME/.cargo/env"
cargo build --release --bin burncloud
```

The client build uses the checked-in generated stylesheet when the Tailwind
standalone binary cannot be downloaded. Install `curl` to regenerate it after
editing `crates/client/input.css` or `tailwind.config.js`.

## Nginx and HTTPS

Keep BurnCloud bound to loopback and terminate TLS in Nginx. The
`X-Forwarded-Proto` header is necessary: it makes the dashboard request a
secure `wss://` LiveView connection instead of an insecure `ws://` connection.

```nginx
server {
    listen 443 ssl http2;
    server_name burncloud.example.com;

    # ssl_certificate and ssl_certificate_key omitted
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

After changing Nginx, validate the service and reload the proxy:

```bash
sudo nginx -t && sudo systemctl reload nginx
curl -fsS https://burncloud.example.com/health
```
