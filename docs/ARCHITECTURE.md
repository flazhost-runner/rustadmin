# Architecture — RustAdmin

Modular-per-feature admin panel (Rust + Rocket + SeaORM + Tera). Each module under
`src/modules/<m>` owns its layers; modules are mounted explicitly in `src/lib.rs::build_rocket`.

```
Request
  → Route (named, method-aware)
  → Guards: CurrentUser (authn) → Authorized (RBAC) → CsrfProtected (web mutations)
  → Controller (thin: parse + render/JSON)
  → Service (trait I*Service + impl; business logic; returns AppError)
  → SeaORM entity / DB
  ↘ any AppError → its Responder (JSON for /api, flash+redirect for web)
```

## Layers

| Layer | Location | Responsibility |
|-------|----------|----------------|
| Route | `modules/*/routes/*` or `controllers.rs::routes()` | URL + method; collected into `Vec<Route>` |
| Guard | `src/guards`, `src/security` | auth, RBAC, CSRF, method-override, headers |
| Controller | `modules/*/controllers/**` | HTTP orchestration; no business logic |
| Service | `modules/*/services/**` | business logic; `@injectable`-equivalent via managed state |
| Entity | `modules/*/models/*` | SeaORM models; portable column types |
| View | `templates/be/default/**`, `templates/layouts/**` | Tera + Tailwind, rendered via `render_view` |

## Dependency Injection (SOLID-D)

Rocket **managed state** is the container. Services are `Arc<dyn I*Service>` registered in
`build_rocket` and injected into controllers as `&State<Arc<dyn I*Service>>`. Controllers depend
on the **trait**, never the concrete struct.

## RBAC (route-driven)

`src/rbac/registry.rs` is the canonical named-route registry (name, method, path). A permission
is `(name, method, guard)` (`guard` = `api` if name starts `api.`, else `web`). The `Authorized`
guard reverse-looks-up the current `(path, method)` → route name, then checks
`has_access(is_admin, perms, name, method)` (matches name **and** method; Administrator bypasses).
`PermissionService::sync_from_registry` upserts permissions lazily when the Permission page opens.

## Error handling

`src/errors/AppError` is an enum (`NotFound/Conflict/Validation/Unauthorized/Forbidden/BadRequest/
Internal`) implementing `Responder`: `/api/*` → JSON envelope, web → flash + redirect (PRG),
internal details masked in production. Services return `AppError`; `?` converts infra errors
(`DbErr`, `BcryptError`).

## Config (Twelve-Factor)

`src/config/env.rs` is the only env reader (validated, secrets fail-fast in prod). `APP_MODE`
selects Full vs API-only at runtime. DB is dialect-agnostic (`src/db.rs` + SeaORM).

## State & scalability

- Web session = encrypted cookie (stateless → horizontal scaling, no sticky sessions).
- JWT blacklist + rate-limit = `TokenStore` (in-memory default; Redis pluggable).
- Site setting (theme + fields) cached globally (`src/site.rs`), primed at liftoff, invalidated on save.
- Files = `storage/` (local) served at `/storage`; OSS/S3 pluggable.

## Frontend template (landing)

`src/modules/home`: `FeCatalogService` (offline-first curated catalog, server-side search +
windowed pagination, anti-SSRF preview proxy). `/` renders the active template — the pinned
default via the native rich `fe/default` view (bound to Setting), other slugs via proxied HTML.
