# RustAdmin

A bootstrap admin panel built with **Rust + Rocket + SeaORM + Tera** — a native-idiom port of
NodeAdmin. Same concepts (SOLID/DI, route-driven RBAC, central errors, theme + frontend-template
switcher, multi-database), implemented the Rust way.

## Features

- **Auth**: web session (encrypted cookie) + JWT (HS256) with a logout **blacklist**; register;
  password reset (OTP, hashed + expiry).
- **Route-driven RBAC**: permissions are `(named-route, method, guard)`, **auto-synced** from
  the route registry. `Authorized` request guard (authenticate → authorize), Administrator bypass.
- **Security**: CSRF (header/query token), security headers, method-override (PUT/DELETE forms),
  bcrypt passwords, magic-byte upload validation, HTML sanitization (ammonia).
- **Modules**: User / Role / Permission (RBAC) · Setting · Profile · Dashboard · Components
  (UI showcase) · Media (file manager) · Home (public landing + FE template switcher).
- **Theming**: 9 palettes, switchable at runtime (DB-driven, CSS variables, no rebuild).
- **Frontend template switcher**: opentailwind-style catalog (server-side search + windowed
  pagination, iframe-srcdoc thumbnails cached client-side, anti-SSRF preview proxy).
- **Multi-database**: SQLite (dev/test) · MySQL · Postgres — chosen at runtime via env.
- **Variants**: Full (UI + API) or API-only via `APP_MODE` (single codebase, purely-additive).
- **Guardrails**: AGENTS.md + convention `checker` + module generator + CI.

## Quick start

```bash
# 1. configure (copy + edit); dev defaults to SQLite, no DB server needed
cp .env.example .env   # or just run — sqlite file is created automatically

# 2. migrate + seed (creates admin@admin.com / 12345678)
cargo run --bin migrate up

# 3. run (Full = UI + API)
APP_MODE=full cargo run
#   → http://127.0.0.1:8000  (landing)
#   → /auth/login            (admin@admin.com / 12345678)

# API-only
APP_MODE=api cargo run
```

## Environment

Read **only** via `src/config/env.rs` (modules never touch the environment). Key vars:

| Var | Default | Notes |
|-----|---------|-------|
| `APP_MODE` | `full` | `full` (UI+API) or `api` |
| `APP_PORT` | `3000` | the port the server binds; `ROCKET_PORT` overrides it when set |
| `DB_TYPE` | `sqlite` | `sqlite` \| `mysql` \| `postgres` |
| `DATABASE_URL` | — | overrides DB parts if set |
| `DB_HOST/PORT/USERNAME/PASSWORD/DATABASE` | — | when not using `DATABASE_URL` |
| `SESSION_SECRET` / `JWT_SECRET` | — | **required in production** (fail-fast) |
| `BCRYPT_ROUNDS` | `10` | |
| `REDIS_URL`, `MAIL_*`, `OSS_*` | — | optional |

## Multi-database

The same migrations + entities run on SQLite/MySQL/Postgres (portable SeaORM types). Tests use
SQLite in-memory; CI also runs the MySQL + Postgres matrix.

## Testing

```bash
cargo test                                   # unit + integration (SQLite in-memory)
cargo run --bin checker                      # convention gate
cargo clippy --all-targets -- -D warnings    # lint
```

See [`docs/TESTING.md`](docs/TESTING.md).

## Tooling

```bash
cargo run --bin make-module <name>   # scaffold a new module
cargo run --bin add-ui               # upgrade API-only → Full
cargo run --bin migrate <up|down|fresh|refresh|status>
```

## Documentation

- [`AGENTS.md`](AGENTS.md) — development rules (source of truth)
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- [`docs/MODULE_GUIDE.md`](docs/MODULE_GUIDE.md)
- [`docs/TESTING.md`](docs/TESTING.md)
- [`docs/API.md`](docs/API.md)
- [`docs/UI_COMPONENTS.md`](docs/UI_COMPONENTS.md)

## Deployment

Build `cargo build --release`, set `SESSION_SECRET` + `JWT_SECRET` + `DB_*` (or `DATABASE_URL`),
run migrations, then run the binary behind a reverse proxy (TLS terminates there; secure cookies
auto-enable in production). Stateless (session in cookie, files in external storage) → horizontal
scaling.
