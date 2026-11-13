#!/usr/bin/env sh
#
# The browser end-to-end run: the real API on a scratch database, the built client, and two
# headless Chromium pages (apps/client/e2e). No Registre: a fake issuer serves only discovery.
#
#   sh scripts/e2e.sh        builds what it needs, then runs the test
#
# Needs Postgres on 127.0.0.1:5653 (mise run db-up) and Playwright's Chromium. Without Chromium
# it prints why and exits 0 when E2E_OPTIONAL=1, which is how check.sh calls it.

set -eu

cd "$(git rev-parse --show-toplevel)"

[ -d apps/client/node_modules ] || (cd apps/client && bun install --frozen-lockfile >/dev/null)

if ! (cd apps/client && bun e2e/chromium.ts); then
  echo "e2e: Playwright's Chromium is missing; install it with: cd apps/client && bunx playwright-core install chromium" >&2
  [ "${E2E_OPTIONAL:-0}" = "1" ] && exit 0
  exit 1
fi

echo "==> e2e: build"
(cd apps/api && cargo build --locked --bin api --example e2e_seed)
(cd apps/client && bun run build >/dev/null)

echo "==> e2e: run"
(cd apps/client && bun test ./e2e/collab.e2e.ts ./e2e/theme.e2e.ts)
