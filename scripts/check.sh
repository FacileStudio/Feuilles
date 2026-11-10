#!/usr/bin/env sh
#
# The repository quality gate. Reports, never rewrites.
#
#   sh scripts/check.sh      fmt, clippy and tests on the API, the client, then the browser e2e
#
# The database tests need TEST_DATABASE_URL (see docs/development.md). Without it they print a
# skip banner and pass, so this script says so loudly. The e2e run needs it too, and
# Playwright's Chromium; without either it is skipped with a message.

set -eu

cd "$(git rev-parse --show-toplevel)"

if [ -z "${TEST_DATABASE_URL:-}" ]; then
  echo "check: TEST_DATABASE_URL is not set, the database tests will skip" >&2
fi

echo "==> apps/api"
(
  cd apps/api
  cargo fmt --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo test --locked
)

echo "==> apps/client"
(
  cd apps/client
  [ -d node_modules ] || bun install --frozen-lockfile >/dev/null
  bun run check
  bun run test
  bun run build
)

echo "==> e2e"
if [ -z "${TEST_DATABASE_URL:-}" ]; then
  echo "check: TEST_DATABASE_URL is not set, skipping the browser e2e run" >&2
  exit 0
fi
E2E_OPTIONAL=1 E2E_ADMIN_URL="$TEST_DATABASE_URL" sh scripts/e2e.sh
