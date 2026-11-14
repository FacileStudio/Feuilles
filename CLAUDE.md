# Feuilles

Collaborative spreadsheets on IronCalc. Rust API in `apps/api`, SvelteKit client in
`apps/client`. Read `docs/architecture.md` before touching `apps/api/src/hub`.

## Stack

- Rust 1.98.1 (`apps/api/rust-toolchain.toml`), axum 0.8, sqlx 0.9 (runtime queries only, no
  `query!` macros), Postgres 16
- tronc-rs v0.5.1 and porte-rs v0.8.1 by GitHub URL and tag. Both name tronc by the same URL
  and tag, so there is one `tronc::Error` type
- `ironcalc = "=0.9.1"`, and the client's `@ironcalc/wasm` at exactly the same version

## Commands

```sh
mise run db-up                                   # Postgres on 127.0.0.1:5653
export TEST_DATABASE_URL='postgres://postgres:postgres@127.0.0.1:5653/postgres?sslmode=disable'
sh scripts/check.sh                              # fmt, clippy -D warnings, tests, then the client
filet check .                                    # must pass before every commit
cd apps/api && cargo run                         # needs DATABASE_URL
```

If `cargo` is a mise shim and mise refuses the untrusted `mise.toml`, run with
`PATH=~/.cargo/bin:$PATH`.

## Invariants

- **One replica.** Rooms are in memory. `docker-compose.yml` sets `replicas: 1` and a fixed
  `container_name`, so a deploy stops the old container first. Never scale the API.
- **Engine pin.** Snapshots and batches are bitcode of IronCalc's private types. Never bump
  `ironcalc` in a routine update; follow the upgrade procedure in `docs/deployment.md`
  (`api engine-export`, deploy, `api engine-verify`). `ENGINE_VERSION` must match `Cargo.toml`.
- **Reload on reject.** `apply_external_diffs` is not atomic in 0.9.1. When a room refuses a
  batch it drops its model and reloads it from the database. Never keep a model that refused a
  batch.
- **The database is the truth.** A batch is stored before it is broadcast; a failed store closes
  the room with 1011.
- **One gate.** Every workbook route, socket and MCP tool goes through
  `modules/workbooks/service.rs::require`. No access is 404, never 403. No admin bypass.
- **Wire protocol.** `docs/api.md` is the contract with the client. Change both together.
- Every route is in `src/reference/`; `tests/reference.rs` fails otherwise.
- Every route gets an authorization test per role (`tests/authz.rs`).

## Style

No inline comments and no comments in function bodies. Doc comments on public items. Crates
deny `unused` and `dead_code`. Conventional Commits.
