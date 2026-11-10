# Changelog

All notable changes to Feuilles. The format follows Keep a Changelog; versions follow SemVer.

## [0.3.0] - 2026-11-10

### Added

- `/mcp` with six tools: `feuilles_list_workbooks`, `feuilles_get_workbook`,
  `feuilles_read_range` (paged under about 25k tokens), `feuilles_write_cells`,
  `feuilles_create_workbook`, `feuilles_add_sheet`, on `tronc::mcp` with `porte::mcpauth`
- The API reference at `/docs` and `/docs/openapi.json`, with a drift test
- `api engine-export <dir>` and `api engine-verify <dir>` for IronCalc upgrades
- Dockerfile, docker-compose.yml with one replica, and the docs set

## [0.2.0] - 2026-11-10

### Added

- Collaboration rooms: shard threads, one authoritative IronCalc model per open workbook,
  batches stored before they are broadcast, snapshots every 256 batches
- `GET /api/workbooks/{id}/ws` with the binary protocol of `docs/api.md`, including `HELLO` and
  `acked` for idempotent resends, and 1008 closes for refused upgrades
- A refused batch reloads the room from the database
- `PUT /cells` and every server write go through the room, live for collaborators

## [0.1.0] - 2026-11-10

### Added

- The API on tronc-rs and porte-rs: Registre sign-in, sessions, directory sync, deletions
- `/api/spaces`, workbook create, list, get, rename, trash, restore, delete, xlsx import and
  export, CSV export, cell read and write
- One access rule for every workbook route, and a 30-day trash purge
