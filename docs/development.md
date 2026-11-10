# Feuilles — Development

How to run the API locally, run the tests and pass the gate.

## Prerequisites

- Rust 1.98.1 (`apps/api/rust-toolchain.toml`) with rustfmt and clippy
- Docker, for Postgres 16 on `127.0.0.1:5653`
- `filet` 0.18 or later, `lefthook` and `bun` through `mise install`
- Sibling checkouts of `~/Code/Facile/tronc-rs` (tag `v0.3.0`) and `~/Code/Facile/porte-rs`
  (tag `v0.4.0`): until the libraries are pushed, `Cargo.toml` reads them through `file://` URLs

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
`cargo test --locked` in `apps/api`, then `bun run check` and `bun run test` in `apps/client`
when it exists. Without `TEST_DATABASE_URL` the database tests print a skip banner and pass, and
the script warns: read the output before trusting a green run. `lefthook` runs it on pre-push.

## Tests

| File | Covers |
|---|---|
| `src/**` `#[cfg(test)]` | Engine calls, A1 ranges, frames, the outbox, the rate window, config |
| `tests/workbooks.rs` | Create, list, rename, trash, restore, delete, import, export, cells, CSRF |
| `tests/authz.rs` | Every workbook route for each space role, the outsider (404) and anonymous (401) |
| `tests/spaces.rs`, `tests/spaces_store.rs` | `/spaces`, `/auth/*`, porte's spaces conformance suite |
| `tests/directory.rs`, `tests/lifecycle.rs` | Snapshot apply and evictions, erase, the 30-day purge, the porte schema |
| `tests/live_sync.rs` | Two real sockets, API writes live, presence, snapshots, shutdown |
| `tests/live_limits.rs` | Reject and reload, viewer, rate limit, 1009, 1003, 1002, silence, 1013, keepalive |
| `tests/live_access.rs` | Idempotent resend, 1008 refusals, eviction, 1011, the Origin check |
| `tests/durability.rs` | Kills a real `api` process during a burst; every acknowledged batch survives |
| `tests/mcp.rs` | tronc's MCP conformance kit, scopes, live writes, paging |
| `tests/reference.rs`, `tests/upgrade.rs` | The `/docs` drift test, engine export and verify |

Integration tests build the real router against Postgres through `tronc::testdb`. Socket tests
serve it on a random port and drive `tokio-tungstenite` clients.

## The F0 spike

`spikes/collab` is the throwaway crate that proved the collaboration design on IronCalc 0.9.1.
It is not built by the gate; see its README.
