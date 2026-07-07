# Testing — RustAdmin

Every feature has tests (TDD/BDD spirit). Tests run on **SQLite in-memory** for speed; CI also
runs the MySQL + Postgres matrix.

## Kinds

| Kind | Where | What |
|------|-------|------|
| Unit | `#[cfg(test)]` in `src/**` | pure helpers (pagination, ci_like, otp, themes, rbac, jwt, blacklist, media validation) |
| Integration | `tests/db_schema.rs` | migrate + seed + entity CRUD, `desc` reserved word, non-unique name, ci_like |
| Security | `tests/auth_security.rs` | JWT roundtrip/expiry, **blacklist** (login→me 200→logout→401 through a real store), RBAC context |
| Web/API | `tests/access_user.rs`, `tests/access_role_perm.rs`, `tests/admin_pages.rs`, `tests/landing_auth.rs` | canonical tables, method-override (+negative 404), verbose API (+REST 404), setting/profile persistence, login session, fe_preview anti-SSRF |
| Render smoke | `tests/layout_smoke.rs` | chrome renders with theme vars, exact menu, image fallback, components |

## Patterns

- **Real-behaviour stores, not always-smooth mocks** (NodeAdmin lesson): the JWT blacklist test
  drives `InMemoryTokenStore`, which actually expires/persists entries — login → access → logout
  → **401** is verified end-to-end.
- **In-memory DB per test**: `db::connect_in_memory()` + `Migrator::up`. A single pooled
  connection keeps the in-memory DB alive; pass it to `build_rocket_with_db`.
- **Web auth in tests**: set the session via `.private_cookie(Cookie::new("uid", admin_id))`, or
  drive the real login (`POST /auth/login`, `Client::tracked` keeps the cookie). CSRF: set
  `csrf_token` private cookie + `?_csrf=<same>`.

## Commands

```bash
cargo test                                   # everything
cargo test --test access_user                # one suite
cargo run --bin checker                      # convention gate
cargo clippy --all-targets --all-features -- -D warnings
```

> E2E (browser) is run locally — slow/flaky in CI, so it is non-blocking there.

## Manual / Postman

Import [`docs/postman/RustAdmin.postman_collection.json`](postman/RustAdmin.postman_collection.json),
set `base_url` (default `http://localhost:3000`, i.e. `APP_PORT`), run **Auth → login** to capture
`access_token`, then drive the Access CRUD and E2E scenario folders against a running server.
