# UI Components — RustAdmin

Tailwind (CDN, Preflight on) with legacy component classes re-implemented via `@apply` (see
`templates/layouts/be/default/head.tera`). **No Bootstrap.** Live reference: `/admin/v1/components`.

## Theme

9 palettes (Blue default), 4 colors each (`primary/secondary/light/dark`) → CSS variables +
inline Tailwind config. Active theme from `settings.theme`, cached globally and injected by
`render_view`. Use `var(--primary)` / themed classes so a theme change restyles everything.

## Classes (snippets)

```html
<!-- Buttons -->
<button class="btn btn-primary-tw">Primary</button>
<button class="btn btn-success btn-sm">Small</button>
<button class="btn btn-danger">Danger</button>

<!-- Form -->
<label class="form-label fw-semibold">Name</label>
<input class="form-control is-invalid"><div class="invalid-feedback">Required</div>

<!-- Badges / status -->
<span class="badge text-bg-primary">Role</span>
<i class="fas fa-check-circle text-green-500 text-xl"></i>   <!-- Active -->
<i class="fas fa-times-circle text-red-500 text-xl"></i>     <!-- Inactive -->

<!-- Alerts --> <div class="alert alert-primary">Themed</div>

<!-- Dropdown (vanilla, no Bootstrap JS) -->
<div class="dropdown">
  <button class="btn btn-primary dropdown-toggle" data-toggle-dd>Action</button>
  <div class="dropdown-menu dropdown-menu-end"><a class="dropdown-item">Edit</a>
    <div class="dropdown-divider"></div><a class="dropdown-item danger" data-confirm="Sure?">Delete</a></div>
</div>

<!-- Card / pagination -->
<div class="tw-card p-6">…</div>
<ul class="pagination"><li class="page-item active"><a class="page-link">1</a></li></ul>
```

## JavaScript helpers (loaded in `foot.tera`)

- `Toast(message, type)` — `success` | `error` | `info`.
- `Modal.open({title, body, buttons})` / `Modal.close()`.
- `confirmDialog(message)` → `Promise<boolean>`; or `data-confirm="…"` on any link/submit.
- Global **image fallback**: failed/empty `<img>` → context-aware FontAwesome placeholder
  (`fa-user` for avatars, else `fa-image`) — `<img>` is always rendered (no `if` guard).
- **CSRF auto-inject**: non-GET forms get `?_csrf=<token>` appended to their action.
- **Trumbowyg** rich-text (`.trumbowyg-editor`) with a **File Manager** button (uploads via
  `/admin/v1/media/*`, magic-byte validated). Description HTML is sanitized server-side on save.

## Canonical index table

2-row `thead` (filter row + header row), `#checkall` select-all, **Delete Selected**,
`q_page_size`, per-column `q_*` filters, Status as **icon**, roles/method as **badge**, action
**dropdown** (Edit + Delete with `data-confirm`), windowed numeric pagination preserving filters.
Reference: `templates/be/default/access/users/index.tera`.
