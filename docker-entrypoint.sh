#!/bin/sh
# RustAdmin container entrypoint (FlazHost / CapRover).
#   1) Map CapRover's $PORT → ROCKET_PORT (Rocket reads ROCKET_ADDRESS/ROCKET_PORT
#      via rocket::Config::figment(); APP_PORT is only used for display/URLs).
#   2) Generate + persist runtime secrets (SESSION_SECRET / JWT_SECRET /
#      ROCKET_SECRET_KEY) — required in production, app fails fast without them.
#   3) Run DB migrations + seed (idempotent; seeds admin@admin.com / 12345678).
#   4) Exec the server.
set -eu

# ── Port: CapRover injects $PORT (default 80). Rocket listens on ROCKET_PORT. ──
: "${PORT:=80}"
export ROCKET_ADDRESS="${ROCKET_ADDRESS:-0.0.0.0}"
export ROCKET_PORT="${ROCKET_PORT:-$PORT}"
export APP_PORT="${APP_PORT:-$PORT}"

# ── SQLite: ensure the DB file lives on a writable, persistable path. ──
if [ "${DB_TYPE:-sqlite}" = "sqlite" ]; then
  export DB_DATABASE="${DB_DATABASE:-/app/data/rust_admin.sqlite}"
  mkdir -p "$(dirname "$DB_DATABASE")" 2>/dev/null || true
fi

# ── Secrets: env wins; otherwise reuse persisted values; otherwise generate. ──
# ROCKET_SECRET_KEY must be a 256-bit key (64 hex chars) — required by Rocket's
# `secrets` feature in the release profile.
SECRETS_FILE="${SECRETS_FILE:-/app/data/.runtime-secrets}"

gen_hex() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 32
  else
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n'
  fi
}

# Remember platform-provided values, then load any persisted ones as fallback.
ENV_SESSION_SECRET="${SESSION_SECRET:-}"
ENV_JWT_SECRET="${JWT_SECRET:-}"
ENV_ROCKET_SECRET_KEY="${ROCKET_SECRET_KEY:-}"
if [ -f "$SECRETS_FILE" ]; then
  # shellcheck disable=SC1090
  . "$SECRETS_FILE" || true
fi
SESSION_SECRET="${ENV_SESSION_SECRET:-${SESSION_SECRET:-}}"
JWT_SECRET="${ENV_JWT_SECRET:-${JWT_SECRET:-}}"
ROCKET_SECRET_KEY="${ENV_ROCKET_SECRET_KEY:-${ROCKET_SECRET_KEY:-}}"

generated=0
if [ -z "$SESSION_SECRET" ];    then SESSION_SECRET="$(gen_hex)";    generated=1; fi
if [ -z "$JWT_SECRET" ];        then JWT_SECRET="$(gen_hex)";        generated=1; fi
if [ -z "$ROCKET_SECRET_KEY" ]; then ROCKET_SECRET_KEY="$(gen_hex)"; generated=1; fi

if [ "$generated" = "1" ]; then
  echo "[entrypoint] generated missing secret(s); persisting to $SECRETS_FILE"
  if ( umask 077 && printf 'SESSION_SECRET=%s\nJWT_SECRET=%s\nROCKET_SECRET_KEY=%s\n' \
       "$SESSION_SECRET" "$JWT_SECRET" "$ROCKET_SECRET_KEY" > "$SECRETS_FILE" ); then :; else
    echo "[entrypoint] WARN: could not persist secrets (read-only /app/data?) — sessions reset on restart"
  fi
fi
export SESSION_SECRET JWT_SECRET ROCKET_SECRET_KEY

echo "[entrypoint] DB_TYPE=${DB_TYPE:-sqlite} DB_DATABASE=${DB_DATABASE:-} ROCKET_PORT=${ROCKET_PORT} NODE_ENV=${NODE_ENV:-production}"

# ── Migrate + seed (SeaORM Migrator, idempotent; m0007 seeds the admin user).
# A failure here (transient DB, already-migrated edge case) must NOT block boot —
# log and continue so the server can still come up against an existing schema.
echo "[entrypoint] running migrations (./migrate up)..."
if /app/migrate up; then
  echo "[entrypoint] migrations OK"
else
  echo "[entrypoint] WARN: migrate exited non-zero — continuing to start server"
fi

# ── Start Rocket (PID 1 for clean SIGTERM/graceful shutdown). ──
echo "[entrypoint] starting rust_admin on ${ROCKET_ADDRESS}:${ROCKET_PORT}"
exec /app/rust_admin
