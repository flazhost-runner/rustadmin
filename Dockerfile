# ── RustAdmin starter kit · FlazHost PaaS (CapRover) ────────────────────────
# Multi-stage build. SQLite comes from sqlx/libsqlite3-sys (bundled C, compiled
# in-tree) and all TLS is rustls — no OpenSSL, so a musl/alpine build yields a
# fully static binary that runs on a bare alpine runtime.

# 1) Build stage
FROM rust:1-alpine AS build
WORKDIR /src

# C toolchain for the bundled sqlite3 (libsqlite3-sys) and ring (rustls).
RUN apk add --no-cache build-base

# Layer-cache the (long) dependency compile: build a dummy skeleton against the
# real Cargo.toml/Cargo.lock first, so source edits don't recompile all deps.
# (Plain layers only — no BuildKit cache mounts; CapRover uses the legacy builder.)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
 && echo '' > src/lib.rs \
 && echo 'fn main() {}' > src/main.rs \
 && echo 'fn main() {}' > src/bin/migrate.rs \
 && cargo build --release --locked --bin rust_admin --bin migrate

# Real source. Touch every .rs so cargo sees them as newer than the dummy build
# (COPY preserves context mtimes, which can predate the layer above).
COPY . .
RUN find src -name '*.rs' -exec touch {} + \
 && cargo build --release --locked --bin rust_admin --bin migrate \
 && mkdir -p /out \
 && cp target/release/rust_admin target/release/migrate /out/

# 2) Runtime stage
FROM alpine:3.20
WORKDIR /app

# ca-certificates for outbound TLS (frontend-template catalog via reqwest,
# optional S3/SMTP); libcap so the non-root user can bind port 80.
RUN apk add --no-cache ca-certificates tzdata libcap \
 && adduser -D -u 10001 appuser

# Binaries.
COPY --from=build /out/rust_admin /app/rust_admin
COPY --from=build /out/migrate    /app/migrate

# Allow the non-root user to bind the privileged port 80 (CapRover default).
RUN setcap 'cap_net_bind_service=+ep' /app/rust_admin

# Disk assets read at runtime, resolved via APP_ROOT (src/config/env.rs):
#   templates/  — Tera templates (rocket_dyn_templates)
#   static/     — FileServer mounts /static and /be/default
#   storage/    — FileServer mount /storage + local upload driver
#   public/fe   — frontend-template catalog cache (written at runtime)
COPY --from=build /src/templates /app/templates
COPY --from=build /src/static    /app/static
COPY --from=build /src/public    /app/public

COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh \
 && mkdir -p /app/data /app/storage \
 && chown -R appuser:appuser /app

# ── Zero-config defaults (all overridable via env) ──────────────────────────
# NODE_ENV=production → the app does NOT self-migrate (entrypoint runs
# ./migrate first) and requires SESSION_SECRET/JWT_SECRET — the entrypoint
# generates and persists them under /app/data when not provided.
ENV NODE_ENV=production \
    APP_ROOT=/app \
    APP_NAME=RustAdmin \
    APP_MODE=full \
    APP_HOST=http://localhost \
    PORT=80 \
    DB_TYPE=sqlite \
    DB_DATABASE=/app/data/rust_admin.sqlite \
    STORAGE_DRIVER=local \
    STORAGE_BASE_PATH=/app/storage \
    ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_LOG_LEVEL=normal

USER appuser
EXPOSE 80
ENTRYPOINT ["/app/docker-entrypoint.sh"]
