# ── Stage 1: Build ──────────────────────────────────────────────
FROM rust:1.80-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY crates/client/Cargo.toml crates/client/Cargo.toml
COPY crates/client/crates/client-dashboard/Cargo.toml crates/client/crates/client-dashboard/Cargo.toml
COPY crates/client/crates/client-finance/Cargo.toml crates/client/crates/client-finance/Cargo.toml
COPY crates/client/crates/client-shared/Cargo.toml crates/client/crates/client-shared/Cargo.toml
COPY crates/common/Cargo.toml crates/common/Cargo.toml
COPY crates/database/Cargo.toml crates/database/Cargo.toml
COPY crates/database/crates/billing/Cargo.toml crates/database/crates/billing/Cargo.toml
COPY crates/database/crates/router/Cargo.toml crates/database/crates/router/Cargo.toml
COPY crates/database/crates/user/Cargo.toml crates/database/crates/user/Cargo.toml
COPY crates/router/Cargo.toml crates/router/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY crates/service/Cargo.toml crates/service/Cargo.toml
COPY crates/service/crates/billing/Cargo.toml crates/service/crates/billing/Cargo.toml
COPY crates/service/crates/router-log/Cargo.toml crates/service/crates/router-log/Cargo.toml
COPY crates/service/crates/user/Cargo.toml crates/service/crates/user/Cargo.toml

# Create dummy source files so cargo can resolve the workspace
RUN mkdir -p crates/client/crates/client-dashboard/src && echo "fn main(){}" > crates/client/crates/client-dashboard/src/main.rs
RUN mkdir -p crates/client/crates/client-finance/src && echo "fn main(){}" > crates/client/crates/client-finance/src/main.rs
RUN mkdir -p crates/client/crates/client-shared/src && touch crates/client/crates/client-shared/src/lib.rs
RUN mkdir -p crates/common/src && touch crates/common/src/lib.rs
RUN mkdir -p crates/database/src && touch crates/database/src/lib.rs
RUN mkdir -p crates/database/crates/billing/src && touch crates/database/crates/billing/src/lib.rs
RUN mkdir -p crates/database/crates/router/src && touch crates/database/crates/router/src/lib.rs
RUN mkdir -p crates/database/crates/user/src && touch crates/database/crates/user/src/lib.rs
RUN mkdir -p crates/router/src && echo "fn main(){}" > crates/router/src/main.rs
RUN mkdir -p crates/server/src && touch crates/server/src/lib.rs
RUN mkdir -p crates/service/src && touch crates/service/src/lib.rs
RUN mkdir -p crates/service/crates/billing/src && touch crates/service/crates/billing/src/lib.rs
RUN mkdir -p crates/service/crates/router-log/src && touch crates/service/crates/router-log/src/lib.rs
RUN mkdir -p crates/service/crates/user/src && touch crates/service/crates/user/src/lib.rs

RUN cargo build --release 2>/dev/null || true

# Now copy the real source and build
COPY . .

RUN cargo build --release

# ── Stage 2: Runtime ───────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/burncloud /usr/local/bin/burncloud

EXPOSE 8080

ENV PORT=8080
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

ENTRYPOINT ["burncloud"]
