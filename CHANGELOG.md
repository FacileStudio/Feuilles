# Changelog

All notable changes to Feuilles. The format follows Keep a Changelog; versions follow SemVer.

## [Unreleased]

## [0.11.1] - 2026-11-14

### Fixed

- Confirm page: the sign-in-again notice shows only for the own-session refusal. Any other 403,
  such as a demotion to viewer after the agent asked, now shows the server's message.

## [0.11.0] - 2026-11-14

### Added

- The Bulle sidebar on a workbook: the header button opens the person's assistant beside the
  grid, framed from `BULLE_URL` (default `https://bulle.facile.studio`), and remembers per person
  whether it was open. The CSP allows that origin in `frame-src`, and `GET /api/auth/session`
  answers it as `bulle_url`
- `/pending`: the page where a person confirms or declines an agent's request, over tronc's
  `PendingPage` on the Go apps' paths: `GET /api/mcp/pending` (`{"actions":[...]}`, at most
  100) and `POST /api/mcp/pending/{id}/confirm|dismiss`. Only a browser login's own session
  cookie reaches it (`porte::mcpauth::require_own_session`); anything else gets 403 and the page
  asks the person to sign in again in their browser, then comes back to the request
- An hourly prune deletes agent actions resolved or expired more than 30 days ago

### Changed

- MCP: `feuilles_delete_sheet`, `feuilles_delete_rows`, `feuilles_delete_columns` and
  `feuilles_clear_range` ask the person before they run, in the client when it can ask, or
  through a `confirm_url` to `/pending`. Asking again for the same edit reuses the open request.
  An unknown sheet, the last sheet, a span out of bounds or a clear over a million cells of data
  is still refused at once, before anyone is asked
- MCP: `feuilles_update_sheet` is marked destructive, like `feuilles_update_workbook`: a rename
  overwrites the tab name. The tools use tronc's annotation presets and pager helpers
- MCP: tool calls stay at 120 a minute per person, with a burst of 120 (was 30), tronc's `App`
  default
- `MCP_RESOURCE` needs `PORTE_TOKEN_KEY`, which keys the confirmations through
  `tronc::mcp::state_key("feuilles", ...)`
- `/pending` and its API answer `frame-ancestors 'none'`; the rest of the client stays framable
  by Docs
- tronc-rs v0.5.1 and porte-rs v0.8.1. A browser login's session token now starts with `b~`:
  cookies minted before this release keep working everywhere but the confirm page
- `docker-compose.yml` drops the swarm-only `deploy.update_config` and `deploy.restart_policy`;
  both services keep `restart: unless-stopped`

### Fixed

- Erasing an account also erases its pending agent actions (`tronc::mcp::erase_pending`)

## [0.10.0] - 2026-11-14

### Added

- Signed-out visitors on `/` get a landing page: muse `LandingHeader` and `LandingFooter`, a
  hero, four feature cards and a closing sign-in. Other signed-out routes keep the sign-in screen
- The hero plays a mise-en-scene demo of a workbook: a cost is typed, `=SUM(B2:B5)` totals the
  column, and the total rolls when a teammate edits a cell. Phones get a compact layout

### Changed

- muse v0.23.0. Text uses Magnetik from muse: `app.css` no longer forces the system font through
  the undefined `--font-fc-sans` token

## [0.9.6] - 2026-11-14

### Added

- The client loads Vision's tracker (`https://vision.facile.studio/s.js`, with `data-mask`) on
  `feuilles.facile.studio` only. The API adds `https://vision.facile.studio` to `script-src`; the
  loader's hash comes with the other inline scripts of the build

## [0.9.5] - 2026-11-13

### Added

- Workbook lists show "1 sheet" or "N sheets" on cards and trash rows again. The API stores
  `workbooks.sheet_count` (migration 0006) with every batch and every new workbook, and returns
  `sheet_count` in every workbook answer, lists included. A boot task counts the workbooks stored
  before it, one at a time, without opening rooms.

### Fixed

- A socket's first ping and session check wait one ping interval instead of running at once, so
  they no longer race the frames right after INIT. This removes a flaky `live_revocation` test.

### Changed

- muse v0.21.2: keyboard moves in menus no longer throw where `scrollIntoView` is missing.

## [0.9.3] - 2026-11-13

### Changed

- muse v0.21: every icon-only control shows a muse tooltip instead of a native `title`, after
  300 ms on hover, at once along the toolbar and at once on keyboard focus. This covers the
  formatting toolbar, the sheet tabs, the back link, the download menu and the workbook card menus
- The workbook cards' and the trash rows' right-click menu is the muse menu: Up and Down move and
  scroll the active entry into view, Home and End jump to the ends, disabled entries are skipped
  and Escape hands focus back to the card. The sheet's right-click menu hands focus back to the
  grid
- Every menu entry has an icon: number formats, rows and columns, and the sheet tab menu

### Fixed

- Trash rows no longer show "0 sheets"

## [0.9.2] - 2026-11-13

### Added

- An icon on every entry of the sheet's right-click menu

### Fixed

- Cell text is readable in the dark theme: black "automatic" text follows the theme text colour,
  white "automatic" fills follow the theme background and black borders follow the text colour,
  and any text colour under 3:1 against its cell falls back to a readable theme colour. Rendering
  only, stored styles do not change
- The sheet redraws in the new colours as soon as the theme changes
- Workbook cards no longer show a "Sheet1" chip, nor "0 sheets" where the list does not know the
  count

## [0.9.0] - 2026-11-13

### Added

- MCP tools to rename, move and trash workbooks, rename, reorder and delete sheets, insert and
  delete rows and columns, and clear ranges, all live through the workbook's room
- Sheets in MCP tools by index or tab name; a missing sheet's error lists the sheets there are
- `feuilles_list_workbooks` pages with `cursor` and `limit`; `feuilles_read_range` stops at the
  sheet's data and answers a `hint` on how to read the rest
- Embed mode: `/w/<id>?embed=1` hides the sidebar and the workbook header except presence, the
  connection hint and an "Open in Feuilles" link. The client's pages may be framed by
  `https://docs.facile.studio` (and `http://localhost:*` outside production)

### Changed

- The workbook header shows who else is here with muse `Presence` and the connection state with
  muse `ConnectionHint`, which shows nothing while the link is live (no more "Live" dot); muse
  v0.20.0
- Each MCP tool declares its own annotations: `write_cells`, `clear_range`, `update_workbook` and
  `delete_workbook` are destructive and idempotent; the delete tools for sheets, rows and columns
  are destructive

## [0.8.3] - 2026-11-13

### Changed

- Deploy: healthchecks run every 30 s with a 1 s start interval, so a deploy cuts over in a few seconds.

## [0.8.2] - 2026-11-13

### Security

- `docker-compose.yml` no longer carries a default Postgres password: compose refuses to start
  without `POSTGRES_PASSWORD`

## [0.8.1] - 2026-11-12

### Fixed

- Explicit pointer cursor for clickable buttons, context menu items, sheet tabs, and workbook cards

## [0.8.0] - 2026-11-12

### Added

- Card grid home view with responsive column tracking and workbook preview thumbnail
- Quick-create card button when the last grid row is incomplete

## [0.7.1] - 2026-11-12

### Added

- Three-dots action icon (`solar:menu-dots-linear`) and right-click context menu on workbook rows in the workbook list and trash

### Security

- porte-rs v0.7.0 and tronc-rs v0.4.0, still one tronc crate. A person Registre deleted can no
  longer sign in again: the login callback, an MCP token's first call and the profile sync
  refuse an erased subject with 403 `permission_denied` before any account is written. If an
  account comes back for an erased subject anyway, the next deletion push or poll erases it
  again

### Fixed

- `api engine-verify` stamps the engine version only on the workbooks it loaded or re-imported.
  A workbook it lost keeps its old version
- The e2e icon check reads the SVG favicon link, now that the head lists two icons

### Documentation

- `docs/architecture.md` links the upstream IronCalc request for inspectable diffs
  (https://github.com/ironcalc/IronCalc/issues/1710), which the room needs to refuse a batch
  before it runs

## [0.7.0] - 2026-11-11

### Changed

- The Feuilles logo is the suite catalog's mark (`solar:table-minimalistic-bold-duotone`), in
  the sidebar and on the sign-in screen

### Fixed

- Favicons, touch and manifest icons, the link preview image and the head metadata come from
  the muse v0.19.0 brand generator, so Feuilles's tab, title, description and og/twitter tags match
  the suite catalog

## [0.6.2] - 2026-11-11

### Security

- porte-rs v0.5.2. It drops `openidconnect` and `rsa` 0.9.10 (RUSTSEC-2023-0071, the Marvin
  timing attack) from the build. Back-channel logout now answers 503 when Registre's keys cannot
  be fetched, instead of 401
- An open workbook socket now ends with its session. The API checks the session at every ping and
  right after any logout or back-channel logout, and closes with 1008 `unauthenticated` when it
  is gone or expired. A person Registre deletes loses every socket in every room at once
- A batch from a person whose account was just deleted is stored without an author instead of
  failing its foreign key and closing the room with 1011 for everyone
- `.xlsx` imports inflate every part through a counted reader before IronCalc reads them, and
  refuse with 413 past 10 000 parts, 128 MiB for one part or 256 MiB in all (zip bombs). At most
  two imports parse at once, and the API container has a 3 GiB memory limit
- A PUSH or API write that takes a workbook past 200 sheets or 5 000 000 filled cells is refused
  (`too_large`) and the room reloads from the database
- One person may hold at most 8 sockets on a workbook, and a workbook at most 200; past that the
  join closes with 1013. The INIT frame now counts against the 8 MiB send budget

### Changed

- A snapshot write never moves a workbook's snapshot back past a newer one, so `engine-export`
  cannot make a workbook unloadable. `docs/deployment.md` now says to stop the API first
- Exports load and write the workbook on a blocking thread, not on an async worker
- The server drops a room's undo history every 20 000 cells written through the API or MCP
- Oversized frames are recognised by tungstenite's error type, not its message text
- The Docker images pin `oven/bun:1.4.2` and `rust:1.98.1-bookworm`; the Postgres password comes
  from `POSTGRES_PASSWORD` (see "The database password" in `docs/deployment.md`)

### Removed

- `spikes/collab`, the F0 spike crate. Its findings are in `docs/architecture.md`

## [0.6.1] - 2026-11-11

### Added

- `favicon.svg`, `favicon.ico`, `apple-touch-icon.png`, `icon-192.png`, `icon-512.png` and
  `site.webmanifest`, drawn from the sidebar's table glyph the way Sablier draws its own. The API
  serves them from the built client; the e2e checks each one
- A "Page not found" page for unknown addresses, instead of SvelteKit's bare "404 Not Found"

### Fixed

- The grid's selection and fill outlines are CSS outlines, not borders
- A workbook that cannot be opened no longer shows an empty download menu, and its message keeps
  a margin from the window edge
- On phones: the workbook title is no longer cut to "Budget …" (the back button shows its arrow
  only), toasts sit above the bottom navigation, and Settings > Advanced stacks each version
  under its label instead of wrapping "IronCalc 0.9.1"

## [0.6.0] - 2026-11-11

### Added

- Import a `.csv` as a new one-sheet workbook, from the workbook list or with `api import-xlsx`.
  The importer handles a UTF-8 BOM or Windows-1252 bytes, `,`/`;`/tab delimiters (sniffed, or
  Excel's `sep=` line), quoted line breaks and French decimal commas. Formulas and digit
  identifiers such as `007` or `+33612345678` stay text. See "CSV files" in `docs/api.md`
- The sidebar user card shows the Registre profile picture for everyone: the API keeps the
  `picture` claim as the account avatar at sign-in and profile sync, and the client syncs the
  profile on load

### Changed

- The CSV export prefixes a `'` to text cells starting with `= + - @ '`, tab or CR (CSV injection
  guard). Numbers are unchanged

### Fixed

- "This sheet as CSV" downloads the selected sheet, not the one selected when the page opened

## [0.5.0] - 2026-11-11

### Added

- `api import-xlsx --space <id> [--title <title>] [--dry-run] <file.xlsx>...`: an operator
  command, run inside the container, that imports `.xlsx` files as ownerless workbooks of an
  existing space through the same IronCalc path as `POST /api/workbooks/import`. It refuses an
  unknown or deleted space and never creates one. See "Importing spreadsheets" in
  `docs/deployment.md`

## [0.4.3] - 2026-11-10

### Changed

- Licensed under AGPL-3.0-only, like Capsule and Plume.

## [0.4.2] - 2026-11-10

### Changed

- tronc-rs v0.3.0 and porte-rs v0.5.1 come from their public GitHub repositories, not local
  `file://` paths, so the image builds anywhere. `cargo tree -i tronc` lists one tronc
- `docs/deployment.md` explains how to bump a library tag instead of the one-off first-deploy
  edits

### Fixed

- The Docker dependency-cache stage builds again: it creates a stub `src/lib.rs` beside the stub
  `main.rs`, and the real build touches both so cargo never ships the stub binary

## [0.4.1] - 2026-11-10

### Fixed

- The browser e2e run waits up to two minutes for the API to become ready

## [0.4.0] - 2026-11-10

### Added

- The SvelteKit client (`apps/client`), merged from `feat/client`: workbook list, trash,
  settings, and the grid on IronCalc's vendored canvas. `check.sh` gates it (`bun run check`,
  `test`, `build`) and filet requires `apps/client` again
- A browser end-to-end run (`mise run e2e`): the real API on a scratch database, the built
  client, two headless Chromium pages; `check.sh` runs it, or skips it with a message when
  Chromium or `TEST_DATABASE_URL` is missing
- The grid announces the selected cell's displayed value to assistive technology

### Changed

- The client speaks the protocol of `docs/api.md`: HELLO with a per-workbook origin kept in
  `sessionStorage` and held with a Web Lock, INIT's `acked` drops batches the server already
  took, `client_seq` keeps increasing across reconnects, and each close code gets its own
  behaviour (1008 stops with "no access" or "sign in again", 1012 reconnects at once, 1009 drops
  the oversized batch). A batch whose echo was lost is never applied twice
- porte-rs v0.5.0: one tronc, no `[patch]`; porte's verifier serves `/mcp` directly and porte's
  `find_by_user` resolves the caller's Registre subject
- The version comes from `apps/client/package.json`, now `0.4.0`

### Fixed

- **Data safety.** A directory snapshot that no longer lists a space used to delete the space
  and, by cascade, all its workbooks; an empty snapshot deleted everything. Now the space is
  marked deleted (migration `0005_space_soft_delete.sql`), its members lose access at once
  (sockets close with 1008), and its workbooks go to the trash, where only the 30-day purge
  deletes them. A space that comes back is undeleted, its workbooks waiting in the trash. An
  empty snapshot while spaces exist is refused and logged at error level

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
