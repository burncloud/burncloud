# Server Architecture

> 📊 **Control Plane - RESTful API & LiveView**

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                     🎛️  SERVER LAYER                                     │
│                                      (crates/server)                                     │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│   ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│   │                           📁 Server Core                                         │    │
│   │                          crates/server/src                                       │    │
│   ├─────────────────────────────────────────────────────────────────────────────────┤    │
│   │                                                                                  │    │
│   │   ┌───────────────────────────────────────────────────────────────────────┐      │    │
│   │   │                              lib.rs                                    │      │    │
│   │   │                                                                        │      │    │
│   │   │   burncloud_server::start_server()                                    │      │    │
│   │   │                                                                        │      │    │
│   │   │   • Axum web framework initialization                                 │      │    │
│   │   │   • LiveView UI support                                               │      │    │
│   │   │   • Static resource serving                                           │      │    │
│   │   │   • WebSocket support                                                 │      │    │
│   │   │   • Session management                                                │      │    │
│   │   │   • Middleware configuration                                          │      │    │
│   │   │                                                                        │      │    │
│   │   └───────────────────────────────────────────────────────────────────────┘      │    │
│   │                                                                                  │    │
│   │   ┌───────────────────────────────────────────────────────────────────────┐      │    │
│   │   │                           bootstrap.rs                                 │      │    │
│   │   │                                                                        │      │    │
│   │   │   ensure_master_key() → MasterKeySource                               │      │    │
│   │   │                                                                        │      │    │
│   │   │   • Priority: env var > key file > auto-generate                      │      │    │
│   │   │   • Reads MASTER_KEY from environment if set                          │      │    │
│   │   │   • Falls back to $XDG_CONFIG_HOME/burncloud/master.key               │      │    │
│   │   │   • Generates 32-byte random key (hex-encoded) on first run           │      │    │
│   │   │   • Writes key file with mode 0600 (Unix)                             │      │    │
│   │   │   • Sets MASTER_KEY env var for downstream consumers                  │      │    │
│   │   │   • Errors hard on write failure — no silent in-memory fallback       │      │    │
│   │   │                                                                        │      │    │
│   │   └───────────────────────────────────────────────────────────────────────┘      │    │
│   │                                                                                  │    │
│   │   ┌───────────────────┐                                                         │    │
│   │   │     📂 api/       │◄─────────────────────────────────────────────────────    │    │
│   │   └───────────────────┘                                                         │    │
│   │                                                                                  │    │
│   └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 📁 API Layer

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                  🌐 RESTful API Layer                                    │
│                              crates/server/src/api                                       │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│   ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│   │                              📄 mod.rs                                           │    │
│   │                                                                                  │    │
│   │   Route definitions and API module organization                                  │    │
│   │                                                                                  │    │
│   │   Routes:                                                                        │    │
│   │   ├── /api/auth/*         → Authentication endpoints                             │    │
│   │   ├── /api/users/*        → User management                                     │    │
│   │   ├── /api/tokens/*       → Token management                                    │    │
│   │   ├── /api/channels/*     → Channel management                                  │    │
│   │   ├── /api/groups/*       → Group management                                    │    │
│   │   ├── /api/logs/*         → Log queries                                         │    │
│   │   └── /api/monitor/*      → Monitoring endpoints                                │    │
│   │                                                                                  │    │
│   └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                          │
│   ┌───────────────────────┐   ┌───────────────────────┐   ┌───────────────────────┐      │
│   │      🔐 auth.rs       │   │      👤 user.rs       │   │      🔑 token.rs      │      │
│   ├───────────────────────┤   ├───────────────────────┤   ├───────────────────────┤      │
│   │                       │   │                       │   │                       │      │
│   │  Authentication       │   │  User Management      │   │  Token Management     │      │
│   │                       │   │                       │   │                       │      │
│   │  POST /login          │   │  GET    /users        │   │  GET    /tokens       │      │
│   │  POST /logout         │   │  POST   /users        │   │  POST   /tokens       │      │
│   │  POST /register       │   │  GET    /users/:id    │   │  DELETE /tokens/:id   │      │
│   │  GET  /me             │   │  PUT    /users/:id    │   │  PUT    /tokens/:id   │      │
│   │  POST /refresh        │   │  DELETE /users/:id    │   │                       │      │
│   │                       │   │                       │   │                       │      │
│   │  • JWT validation     │   │  • User CRUD          │   │  • API key CRUD       │      │
│   │  • Session handling   │   │  • Password hashing   │   │  • Token generation   │      │
│   │  • Permission checks  │   │  • Profile updates    │   │  • Scope management   │      │
│   │                       │   │                       │   │                       │      │
│   └───────────────────────┘   └───────────────────────┘   └───────────────────────┘      │
│                                                                                          │
│   ┌───────────────────────┐   ┌───────────────────────┐   ┌───────────────────────┐      │
│   │    📡 channel.rs      │   │      👥 group.rs      │   │       📋 log.rs       │      │
│   ├───────────────────────┤   ├───────────────────────┤   ├───────────────────────┤      │
│   │                       │   │                       │   │                       │      │
│   │  Channel Management   │   │  Group Management     │   │  Log Management        │      │
│   │                       │   │                       │   │                       │      │
│   │  GET    /channels     │   │  GET    /groups       │   │  GET    /logs         │      │
│   │  POST   /channels     │   │  POST   /groups       │   │  GET    /logs/:id     │      │
│   │  GET    /channels/:id │   │  GET    /groups/:id   │   │  DELETE /logs/:id     │      │
│   │  PUT    /channels/:id │   │  PUT    /groups/:id   │   │  GET    /logs/stats   │      │
│   │  DELETE /channels/:id │   │  DELETE /groups/:id   │   │                       │      │
│   │                       │   │                       │   │                       │      │
│   │  • Upstream config    │   │  • Group CRUD         │   │  • Log queries        │      │
│   │  • Protocol settings  │   │  • Member management  │   │  • Filtering          │      │
│   │  • Health monitoring  │   │  • Permission sets    │   │  • Statistics         │      │
│   │  • Load balancing     │   │                       │   │  • Export             │      │
│   │                       │   │                       │   │                       │      │
│   └───────────────────────┘   └───────────────────────┘   └───────────────────────┘      │
│                                                                                          │
│   ┌───────────────────────────────────────────────────────────────────────────────┐      │
│   │                              📊 monitor.rs                                      │      │
│   ├───────────────────────────────────────────────────────────────────────────────┤      │
│   │                                                                                │      │
│   │  System Monitoring                                                             │      │
│   │                                                                                │      │
│   │  GET  /monitor/health      → Health check endpoint                             │      │
│   │  GET  /monitor/metrics     → System metrics (CPU, Memory, Disk)               │      │
│   │  GET  /monitor/status      → Service status                                    │      │
│   │  GET  /monitor/channels    → Channel health status                             │      │
│   │                                                                                │      │
│   │  • Real-time metrics                                                          │      │
│   │  • Channel health checks                                                      │      │
│   │  • Resource utilization                                                       │      │
│   │  • Alert integration                                                          │      │
│   │                                                                                │      │
│   └───────────────────────────────────────────────────────────────────────────────┘      │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 🔄 Server Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                 📊 Server Request Flow                                   │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│                                                                                          │
│   ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐      │
│   │  HTTP    │     │ Middleware│    │  Router  │     │  Handler │     │ Service  │      │
│   │ Request  │────▶│  Layer   │────▶│  (Axum)  │────▶│  (API)   │────▶│  Layer   │      │
│   └──────────┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘      │
│                          │                                                   │           │
│                          │                                                   │           │
│                          ▼                                                   ▼           │
│                    ┌──────────┐                                        ┌──────────┐     │
│                    │  Auth    │                                        │ Database │     │
│                    │ Check    │                                        │  Layer   │     │
│                    └──────────┘                                        └──────────┘     │
│                                                                                          │
│                                                                                          │
│   Middleware Stack:                                                                      │
│   ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│   │  1. CORS Middleware         → Cross-origin resource sharing                      │    │
│   │  2. Logging Middleware      → Request/response logging                          │    │
│   │  3. Auth Middleware         → JWT validation                                    │    │
│   │  4. Rate Limiting           → Request throttling                                │    │
│   │  5. Compression             → Response compression                              │    │
│   └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Server Components

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                 🛠️  Server Components                                    │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│   ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│   │                              🔌 LiveView Support                                │    │
│   ├─────────────────────────────────────────────────────────────────────────────────┤    │
│   │                                                                                  │    │
│   │   • Real-time UI updates via WebSocket                                          │    │
│   │   • Server-side rendering                                                       │    │
│   │   • Interactive dashboard support                                               │    │
│   │   • State synchronization                                                       │    │
│   │                                                                                  │    │
│   └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                          │
│   ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│   │                              💾 Session Management                              │    │
│   ├─────────────────────────────────────────────────────────────────────────────────┤    │
│   │                                                                                  │    │
│   │   • JWT-based authentication                                                    │    │
│   │   • Token refresh mechanism                                                     │    │
│   │   • Session storage (Redis/In-memory)                                           │    │
│   │   • Secure cookie handling                                                      │    │
│   │                                                                                  │    │
│   └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                          │
│   ┌─────────────────────────────────────────────────────────────────────────────────┐    │
│   │                              📁 Static Resources                                │    │
│   ├─────────────────────────────────────────────────────────────────────────────────┤    │
│   │                                                                                  │    │
│   │   • CSS/JS assets serving                                                       │    │
│   │   • Image serving                                                               │    │
│   │   • SPA fallback routing                                                        │    │
│   │   • Cache headers                                                               │    │
│   │                                                                                  │    │
│   └─────────────────────────────────────────────────────────────────────────────────┘    │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 📚 Sub-Documentation

| Module | Description | Link |
|--------|-------------|------|
| API | RESTful API endpoints | [api/README.md](./api/README.md) |
