# Feuilles

[![ci](https://github.com/FacileStudio/Feuilles/actions/workflows/ci.yml/badge.svg)](https://github.com/FacileStudio/Feuilles/actions/workflows/ci.yml)

Collaborative spreadsheets for the Facile suite, on the IronCalc engine, in the browser and on
the server.

Feuilles replaces Grist for plain spreadsheets. People in a Registre space edit a workbook
together, agents read and write cells through MCP, and xlsx goes in and out. It is built
locally and not deployed yet.

## What it does

- Workbooks per person or per Registre space, with a 30-day trash
- Live co-editing over one WebSocket per workbook, ordered and stored by the server
- The same IronCalc engine computes in the browser (WASM) and on the server
- Import and export `.xlsx`, import a `.csv` as a new workbook, export a sheet as CSV
- Read and write cell ranges over HTTP, live for everyone with the workbook open
- Fifteen MCP tools for agents, with Registre tokens, free on every plan
- Registre sign-in only, with spaces and account deletions synced from Registre

## Stack

| Layer | Tech |
|---|---|
| API | Rust 1.98, axum 0.8, sqlx 0.9, PostgreSQL 16, IronCalc 0.9.1, tronc-rs 0.3.0, porte-rs 0.5.2 |
| Client | SvelteKit 2 with Svelte 5 runes, `@ironcalc/wasm` 0.9.1, muse (`@facile/muse`) |
| Deploy | Docker Compose, one container and one replica behind Traefik |

## Quick start

```sh
mise install
mise run db-up
cd apps/api
DATABASE_URL='postgres://postgres:postgres@127.0.0.1:5653/feuilles?sslmode=disable' PORT=4000 cargo run
```

Then open <http://localhost:4000/docs> for the API reference.

### Tests and the gate

```sh
export TEST_DATABASE_URL='postgres://postgres:postgres@127.0.0.1:5653/postgres?sslmode=disable'
sh scripts/check.sh
filet check .
```

## Configuration

| Variable | What it does |
|---|---|
| `DATABASE_URL` | Postgres connection string |
| `PORT` | Listen port; the image sets `4000` |
| `APP_ENV` | `production` requires the next two |
| `OIDC_ISSUER` | Registre, `https://sso.facile.studio`, with `OIDC_CLIENT_ID` and `OIDC_CLIENT_SECRET` |
| `PORTE_TOKEN_KEY` | Seals stored OIDC tokens |
| `MCP_RESOURCE` | `https://feuilles.facile.studio/mcp`; mounts `/mcp` |
| `BULLE_URL` | Optional; the Bulle sidebar origin, `https://bulle.facile.studio` by default |

Full reference: [docs/configuration.md](docs/configuration.md).

## Structure

```
apps/
  api/       Rust API: auth (porte), engine (IronCalc), hub (rooms), modules, mcp, tests
  client/    SvelteKit client, served by the API from CLIENT_DIR
docs/        Architecture, configuration, development, deployment, API
scripts/     The gate, the dev database, the version stamp, the deploy check
```

## Documentation

| Doc | What's in it |
|---|---|
| [Architecture](docs/architecture.md) | Topology, collaboration rooms, data model, auth, known limits |
| [Configuration](docs/configuration.md) | Every environment variable and default |
| [Development](docs/development.md) | Local setup, tests, the quality gate |
| [Deployment](docs/deployment.md) | Docker image, Compose, one replica, engine upgrades, first deploy |
| [API](docs/api.md) | HTTP routes, the WebSocket protocol, MCP tools |

## License

Feuilles is licensed under the [GNU Affero General Public License v3.0](LICENSE) (`AGPL-3.0-only`). A commercial licence is available from Facile Studio for use that cannot meet the AGPL's terms.

---

Part of the [Facile Suite](https://facile.studio) — self-hosted tools for creative studios
and freelancers. One login, zero cloud dependency.
