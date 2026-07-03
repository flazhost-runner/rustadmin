# Module Guide — RustAdmin

How to add a feature module. Use the `access` module (User/Role/Permission) as the reference.

## Quick scaffold

```bash
cargo run --bin make-module product
```

Generates `src/modules/product/{mod,models,services,controllers}.rs` + a view. Then wire it up
(the generator prints these steps).

## Files & order

1. **Entity** `models/<x>.rs` — `DeriveEntityModel`, `id: String` PK (`auto_increment = false`),
   portable types only (no vendor types). Pin `table_name`.
2. **Migration** `src/migrations/mXXXX_create_<x>.rs` — SeaORM schema builder (reversible), then
   register in `src/migrations/mod.rs`. (Entity ⇒ migration is **required**.)
3. **Service interface + impl** `services/<x>_service.rs` —
   `trait I<X>Service: Send + Sync` + `struct <X>Service`. Use `paginate`/`ci_like`; return
   `AppError` on failure. Re-export from `services/mod.rs`.
4. **Register service (DI)** — in `src/lib.rs::assemble`: `let svc: Arc<dyn I<X>Service> =
   Arc::new(<X>Service); rocket = rocket.manage(svc);`.
5. **Validator** `validators/<x>.rs` — `#[derive(FromForm)]` DTO + `validate*()` returning
   `Result<Input, FormError>` (inline errors + `old`). Required when there is write input.
6. **Controller** `controllers/web/<x>.rs` (+ `api/<x>.rs`) — thin; inject
   `&State<Arc<dyn I<X>Service>>`, `&State<DatabaseConnection>`. Web renders via `render_view`
   + `chrome`/`merge`; API returns the JSON envelope.
7. **Routes** `routes/web.rs` / `routes/api.rs` — `routes![...]`; mount in `build_rocket`.
   Add the named routes to `src/rbac/registry.rs` (names PERSIS the canonical pattern).
8. **Views** `templates/be/default/<x>/{index,create,edit}.tera` — extend
   `layouts/be/default/main`; follow the **canonical index table** (see `access/users/index`).
9. **Test** `tests/<x>.rs` — integration (SQLite in-memory) + api (+ a render smoke). **Required.**
10. **Docs** — update `README.md` and `docs/API.md` (if API).

## Canonical CRUD route names (per resource)

`index` GET · `create` GET · `store` POST · `edit` GET `/<id>/edit` · `update` **PUT**
`/<id>/update` · `delete` **DELETE** `/<id>/delete` · `delete_selected` POST. API is
**verbose & symmetric** (minus `create`). Forms use the method-override fairing (`?_method=`)
and put the CSRF token in the **query**.

## Conventions enforced by the checker

- Service struct ⇒ matching `I*Service` impl (Dependency Inversion).
- Controllers don't construct services (`::new`) — inject from state.
- No `std::env` in modules; no vendor column types; tests must exist.

Run before "done": `cargo run --bin checker && cargo clippy --all-targets -- -D warnings && cargo test`.
