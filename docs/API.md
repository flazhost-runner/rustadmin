# API — RustAdmin

REST API (JWT, HS256). Base: `/api/v1`. All responses use the canonical NodeAdmin envelope
`{ "status": bool, "message": string, "data"?: object|array|null }`. List endpoints nest
pagination inside `data`: `{ "datas": [...], "paginate_data": { total_data, current_page,
page_size, total_page } }`. Errors → same envelope with `status:false` and the appropriate
HTTP status. Paths are **verbose & symmetric** to web (NOT REST).

## Auth

| Method | Path | Body | Notes |
|--------|------|------|-------|
| POST | `/api/v1/auth/login` | `{email, password}` | → `{token, token_type, expires_at, user}` |
| POST | `/api/v1/auth/logout` | — (Bearer) | blacklists the token's `jti` |
| GET  | `/api/v1/auth/me` | — (Bearer) | current user (blacklist-checked) |

Send `Authorization: Bearer <token>` for protected endpoints. Clients send real `PUT`/`DELETE`
(no `?_method` override — that's web forms only).

## Access — User / Role / Permission

For `{resource}` ∈ `user` | `role` | `permission` (base `/api/v1/access/{resource}`):

| Action | Method | Path |
|--------|--------|------|
| index | GET | `/api/v1/access/{resource}?q_page&q_page_size&q_*` |
| store | POST | `/api/v1/access/{resource}/store` |
| edit | GET | `/api/v1/access/{resource}/<id>/edit` |
| update | PUT | `/api/v1/access/{resource}/<id>/update` |
| delete | DELETE | `/api/v1/access/{resource}/<id>/delete` |
| delete_selected | POST | `/api/v1/access/{resource}/delete_selected` — `{selected:[id,...]}` |

REST-style paths (`GET /:id`, `PUT /:id`, …) intentionally **404**.

### Bodies

- User store/update: `{code, name, email, password, phone?, timezone?, status?, blocked?, blocked_reason?, roles:[id]}`
- Role store/update: `{name, status?, desc?}`
- Permission store/update: `{name, guard_name?, method?, status?, desc?}`

## Role → Permission management (symmetric to web)

| Action | Method | Path |
|--------|--------|------|
| list | GET | `/api/v1/access/role/<id>/permission?q_*` |
| assign (one) | GET | `/api/v1/access/role/<id>/permission/<permission_id>/assign` |
| assign bulk | POST | `/api/v1/access/role/<id>/permission/assign_selected` — `{selected:[...]}` |
| unassign (one) | GET | `/api/v1/access/role/<id>/permission/<permission_id>/unassign` |
| unassign bulk | POST | `/api/v1/access/role/<id>/permission/unassign_selected` — `{selected:[...]}` |

## Authorization

Every access endpoint is RBAC-gated by `(route-name, method)`. Administrator bypasses. The
permission set is auto-synced from the route registry (open the Permission page, or call the
permission index).

> A missing `<id>` on `edit`/`update`/`delete` returns **404** (canonical error envelope), never
> `200` with an empty body.

## Postman

Import [`docs/postman/RustAdmin.postman_collection.json`](postman/RustAdmin.postman_collection.json).
Set the `base_url` collection variable (default `http://localhost:3000`, i.e. `APP_PORT`), run
**Auth → login** to capture `access_token`, then exercise the Access and E2E scenario folders.
