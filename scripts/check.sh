#!/usr/bin/env sh
#
# The repository quality gate. Reports, never rewrites.
#
#   sh scripts/check.sh      fmt, clippy and tests on the API, then the client
#
# The database tests need TEST_DATABASE_URL (see docs/development.md). Without it they print a
# skip banner and pass, so this script says so loudly.

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
if [ ! -f apps/client/package.json ]; then
  echo "check: apps/client is not on this branch, skipping the client" >&2
  exit 0
fi
(
  cd apps/client
  [ -d node_modules ] || bun install --frozen-lockfile >/dev/null
  bun run check
  bun run test
)
