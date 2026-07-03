# AGENTS.md — RustAdmin Development Rules (for AI & developers)

> **Single source of truth.** Every AI (Claude Code, Cursor, Copilot) and developer MUST
> follow this when adding/changing code. Consistency is enforced by the convention checker
> (`cargo run --bin checker`) as a CI gate — deviations are rejected.

RustAdmin is a **bootstrap** admin panel (Rust + Rocket + SeaORM + Tera), a native-idiom port
of NodeAdmin. Read also: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md),
[`docs/MODULE_GUIDE.md`](docs/MODULE_GUIDE.md), [`docs/TESTING.md`](docs/TESTING.md).

## Request lifecycle

```
Route (named, method-aware)
  → guards: CurrentUser (authenticate) → Authorized (RBAC) → CsrfProtected (web mutations)
  → controller (#[get]/#[post]/#[put]/#[delete]) — thin: parse + render/JSON, NO business logic
  → service (trait I*Service + struct, returns AppError on failure)
  → SeaORM entity / DB
  ↘ any AppError → its Responder (JSON for /api, flash+redirect for web)
```

## Mandatory principles

1. **SOLID / DI.** Services implement an interface (`trait I*Service: Send + Sync`) and are
   shared as **managed state** (`State<Arc<dyn I*Service>>`) — Rocket's managed state IS the
   DI container. Controllers depend on the **trait**, never construct services with `::new`.
2. **DRY.** Reuse helpers: `helpers::pagination` (`PageParams`/`PaginationMeta`/`page_window`),
   `helpers::ci_like` (case-insensitive LIKE), `helpers::view::render_view`, `helpers::flash`
   (PRG errors/old), `helpers::response`, `security::*`, `guards::*`.
3. **Central errors.** Services **return `AppError`** (Rust's idiom for "throw"). Controllers
   never format error responses — `AppError`'s `Responder` does it. Forbidden: returning a
   non-`AppError` error type to the HTTP layer.
4. **Separation of concerns.** Controller ≠ Service ≠ Entity ≠ View. Business logic only in
   services.
5. **Config only via `crate::config`.** **Never** read `std::env` inside `src/modules/*`.
6. **DB portability (multi-DB, not just the ORM).**
   - Entities: abstract types only. **No** vendor types (`longtext`/`mediumtext`/vendor
     `datetime`/`collation`/`tinyint`). `id` = `String` (varchar(36) UUID), status = varchar.
   - Migrations: SeaORM schema builder (Table/ColumnDef), **not** raw vendor SQL.
   - Queries: use `ci_like()` (LOWER LIKE LOWER), never raw `LIKE` with vendor case rules.
   - Tests run on SQLite in-memory (proves portability); CI also runs MySQL/Postgres.

## Before coding: present the artifact plan

When asked to build a feature/module, conclude the artifacts (matrix below) and **present a
plan**; ask only if ambiguous (UI vs API-only? read-only vs CRUD? need API?). Then follow
[`docs/MODULE_GUIDE.md`](docs/MODULE_GUIDE.md).

## Artifact matrix

**Always** (service-backed module): Service + `I*Service`, Controller, ≥1 Route, **Test**,
docs update. **Conditional**: Entity (stores data) → Migration **required**; write input
(store/update) → Validator **required**; UI → web route + view **required**; API → api test +
`docs/API.md` entry **required**. **TEST is mandatory for any feature.**

## Canonical replication rules (UI)

- Named routes + methods PERSIS NodeAdmin (`{admin.v1|web|api.v1}.{module}.{resource}.{action}`;
  access namespace `access` + **singular**). Registered in `src/rbac/registry.rs`.
- `update` = **PUT**, `delete` = **DELETE** via the method-override fairing (`?_method=`).
  API is **verbose & symmetric** to web (NOT REST). Delete is a POST form (+`?_method=DELETE`)
  with CSRF in the **query** (RustAdmin reads CSRF from header/query, never the body).
- Index tables follow the **canonical structure** (2-row thead filter+header, `#checkall`,
  Delete Selected, `q_page_size`, per-column `q_*` filters, Status=icon, badges, action
  dropdown, windowed pagination). Templates are single-extension `.tera`; mark trusted URLs
  (`route()`/`get_file()`) `| safe` (autoescape is on).
- Theme: 9 palettes (exact hex), active theme from `settings.theme`, cached globally
  (`crate::site`) and injected by `render_view` — all views use `var(--primary)` / themed
  classes.

## DO NOT (rejected by CI)

- ❌ `XService::new()` in a controller → inject `&State<Arc<dyn IXService>>`.
- ❌ A `*Service` struct without an `impl I*Service` (Dependency Inversion).
- ❌ Returning a non-`AppError` error to the HTTP layer.
- ❌ Vendor column types / raw vendor SQL in entities/migrations.
- ❌ `std::env::var` in `src/modules/*` → use `crate::config`.
- ❌ Adding a module without a test.

## Definition of Done

- [ ] `cargo run --bin checker` → exit 0.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` → clean.
- [ ] `cargo test` → green (+ new tests for the feature).
- [ ] Security: guards in order (CurrentUser → Authorized), CSRF on web mutations, validated
      input, secrets only from `config`.
- [ ] README / docs/API.md updated.

## Commands

```
cargo run --bin checker            # convention gate (run before "done")
cargo run --bin migrate up         # migrations (up|down|fresh|refresh|status)
cargo run --bin make-module <name> # scaffold a new module
cargo run --bin add-ui             # upgrade API-only → Full
APP_MODE=full cargo run            # run the app (full UI+API)
APP_MODE=api  cargo run            # run API-only (web layer skipped)
cargo test                         # all tests (unit + integration)
```
