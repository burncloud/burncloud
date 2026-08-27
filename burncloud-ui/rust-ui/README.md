# BurnCloud Rust UI

This subproject is the Rust migration of the BurnCloud UI workbench. The first
three buyer pages use the real BurnCloud management APIs and require an active
BurnCloud backend.

The migrated protected routes are:

- `GET /buyer/overview`
- `GET /buyer/playground`
- `GET /buyer/marketplace`

`/buyer`, `/supplier`, and `/admin` are also protected workspace entry points.
Unauthenticated requests are redirected to `/login` and return to their original
path after a successful login.

## Run

Start the BurnCloud backend first. Its management API must include the
authenticated `GET /api/models/catalog` endpoint. Then, from the `burncloud-ui`
repository root:

```powershell
$env:BURNCLOUD_API_BASE = "http://127.0.0.1:3002"
cargo run --manifest-path rust-ui/Cargo.toml
```

Open `http://127.0.0.1:3001/buyer/overview`,
`http://127.0.0.1:3001/buyer/playground`, or
`http://127.0.0.1:3001/buyer/marketplace`.

Use a BurnCloud account on `/login`. The UI stores the backend JWT in an
HttpOnly session cookie; it never sends the Console JWT or a data-plane API key
to browser JavaScript. Playground inference is proxied by the Rust UI through
the authenticated backend API.

The login page and authenticated workspace support English, Simplified Chinese,
Traditional Chinese, and Japanese. The selected language is stored in browser
local storage as `burncloud_selected_language`, persists across navigation and
reloads, and never changes values received from or sent to backend APIs.

Set a different UI port with `BURNCLOUD_UI_PORT`. Set a different backend URL
with `BURNCLOUD_API_BASE` (default: `http://127.0.0.1:3002`). The legacy React
source remains in the repository as the reference for pages that have not yet
been migrated.
