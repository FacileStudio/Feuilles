# Feuilles — Development

How to run the API locally, run the tests and pass the gate.

## Prerequisites

- Rust 1.98.1 (`apps/api/rust-toolchain.toml`) with rustfmt and clippy
- Docker, for Postgres 16 on `127.0.0.1:5653`
- `filet` 0.18 or later, `lefthook` and `bun` through `mise install`
- Network access to `github.com/FacileStudio`: `Cargo.toml` fetches tronc-rs (tag `v0.3.0`)
  and porte-rs (tag `v0.5.1`) by git URL

## Setup

```sh
mise install
mise run db-up
export TEST_DATABASE_URL='postgres://postgres:postgres@127.0.0.1:5653/postgres?sslmode=disable'
```

`db-up` starts `feuilles_dev_postgres` and returns once `pg_isready` answers.

## Run

```sh
cd apps/api
DATABASE_URL='postgres://postgres:postgres@127.0.0.1:5653/feuilles?sslmode=disable' PORT=4000 cargo run
```

Without `OIDC_ISSUER` nobody can sign in; tests mint sessions with porte's `Manager::issue`.
Subcommands of the same binary: `api healthcheck`, `api migrate status|up|version|baseline N`,
`api engine-export <dir>`, `api engine-verify <dir>`.

## The gate

```sh
sh scripts/check.sh
filet check .
```

`check.sh` runs `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and
`cargo test --locked` in `apps/api`, then `bun run check`, `bun run test` and `bun run build` in
`apps/client`, then the browser e2e run below. Without `TEST_DATABASE_URL` the database tests
print a skip banner and pass, the e2e run is skipped, and the script warns: read the output
before trusting a green run. Without Playwright's Chromium the e2e run is skipped with a
message. `lefthook` runs it on pre-push.

## The browser end-to-end run

```sh
mise run e2e          # or: sh scripts/e2e.sh, with Postgres already up
```

`scripts/e2e.sh` builds `apps/api` (the `api` binary and the test-only `e2e_seed` example) and
the client, then runs `apps/client/e2e/collab.e2e.ts` with `bun test`. The harness
(`apps/client/e2e/stack.ts`):

1. drops and recreates the database `feuilles_e2e` on `E2E_ADMIN_URL` (default
   `postgres://postgres:postgres@127.0.0.1:5653/postgres`);
2. serves a fake issuer that answers OIDC discovery and an empty key set, nothing else, so porte
   boots with `OIDC_ISSUER` and Registre subjects key space membership. It cannot sign anyone in,
   and the directory sync against it fails with a warning and changes nothing;
3. starts `target/debug/api` on a free port with `CLIENT_DIR=apps/client/build`, and waits up to
   `E2E_READY_SECONDS` (default 120) for `/ready`: on a loaded machine the first migrations alone
   can take over 30 seconds;
4. runs `e2e_seed` (`apps/api/examples/e2e_seed.rs`), which writes alice (space owner), bob
   (member) and an outsider straight into the database and mints each a browser session with
   porte's session store. The `api` binary has no login bypass: the image builds only `api`.

The test opens one workbook as alice and bob in two headless Chromium contexts. Alice types `1`,
`2`, `3` in A1 to A3, then `=SUM(A1:A3)` in A4 while bob sets A2 to `10`; both pages must show
`14` in A4 within one second. Alice reloads and still sees it. The outsider gets the no-access
screen. The pages read cell values from the grid's accessible `Value of the selected cell`
output.

Playwright is `playwright-core` 1.64.1, whose Chromium (revision 1248) is the one installed on the
dev machine. Install it elsewhere with `cd apps/client && bunx playwright-core install chromium`.

## Tests

| File | Covers |
|---|---|
| `src/**` `#[cfg(test)]` | Engine calls, A1 ranges, frames, the outbox, the rate window, config |
| `tests/workbooks.rs` | Create, list, rename, trash, restore, delete, import, export, cells, CSRF |
| `tests/authz.rs` | Every workbook route for each space role, the outsider (404) and anonymous (401) |
| `tests/spaces.rs`, `tests/spaces_store.rs` | `/spaces`, `/auth/*`, porte's spaces conformance suite |
| `tests/directory.rs`, `tests/lifecycle.rs` | Snapshot apply and evictions, erase, the 30-day purge, the porte schema |
| `tests/directory_safety.rs` | A dropped space is soft-deleted and its workbooks trashed, an empty snapshot is refused, a space comes back, members lose access at once |
| `tests/live_sync.rs` | Two real sockets, API writes live, presence, snapshots, shutdown |
| `tests/live_limits.rs` | Reject and reload, viewer, rate limit, 1009, 1003, 1002, silence, 1013, keepalive |
| `tests/live_access.rs` | Idempotent resend, 1008 refusals, eviction, 1011, the Origin check |
| `tests/durability.rs` | Kills a real `api` process during a burst; every acknowledged batch survives |
| `tests/mcp.rs` | tronc's MCP conformance kit, scopes, live writes, paging |
| `tests/reference.rs`, `tests/upgrade.rs` | The `/docs` drift test, engine export and verify |

Each test in `directory_safety.rs` owns its own schema (`common::state_in`), because the
directory version is one row per database.

Integration tests build the real router against Postgres through `tronc::testdb`. Socket tests
serve it on a random port and drive `tokio-tungstenite` clients.

## The F0 spike

`spikes/collab` is the throwaway crate that proved the collaboration design on IronCalc 0.9.1.
It is not built by the gate; see its README.
