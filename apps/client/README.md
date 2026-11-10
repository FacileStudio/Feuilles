# Feuilles client

The browser side of Feuilles: SvelteKit 2 with adapter-static, Svelte 5 runes, Tailwind 4 and `@facile/muse` v0.17.1, on bun. The API serves the build from `CLIENT_DIR`.

## Run it

Against the real API (plan section 2), which listens on `127.0.0.1:4000`:

```sh
bun install --frozen-lockfile
bun run dev
```

Vite proxies `/api` (WebSocket included) and `/version` to `http://127.0.0.1:4000`. Set `FEUILLES_API` to point the proxy elsewhere.

Without the API, run the dev mock in a second terminal. It lives in `src/lib/dev/`, which no production module imports:

```sh
MOCK_PORT=4600 bun src/lib/dev/mock-api.ts
FEUILLES_API=http://127.0.0.1:4600 bun run dev
```

The mock answers the session, spaces, workbook and `/version` routes, and runs one in-memory room per workbook with the real WASM engine. `MOCK_ENGINE=0.8.0` makes it report another engine version, to see the version guard.

## Gate

```sh
bun run check   # svelte-check, zero errors and zero warnings
bun run test    # vitest, run under bun
bun run build   # static build in build/
```

`bun run test` runs `bun --bun vitest run`. vitest 5 needs Node 22.12 or later and this machine has Node 20, so bun is the runtime.

## What is here

| Path | What it does |
|---|---|
| `src/lib/backend.ts`, `src/lib/api/` | Fetch wrapper: relative paths, `credentials: 'include'`, `X-Facile-CSRF: 1` on every non-GET, the `{"error":{"code","message"}}` envelope turned into `ApiError` with a typed `code` |
| `src/lib/engine/protocol.ts`, `protocol-server.ts`, `wire.ts` | The frames of `docs/api.md`, byte for byte. The server-side encoders serve the tests and the mock |
| `src/lib/engine/replica.ts` | The 7.1 algorithm: `confirmed`, `view`, `pending`, the reject path, and the F0 notes (no empty batch; a pending batch that no longer replays is dropped) |
| `src/lib/engine/session.svelte.ts` | Socket, HELLO, INIT with `acked`, reconnect per close code and idempotent resend, presence, cursor frames at most 10 per second |
| `src/lib/engine/version.ts` | The engine version guard |
| `src/lib/grid/canvas/` | IronCalc v0.9.1's `WorksheetCanvas` vendored under `vendor/`, see `NOTICE` |
| `src/lib/grid/*.svelte` | `Grid`, `CellEditor`, `FormulaBar`, `Toolbar`, `SheetTabs`, `ContextMenu`, `Cursors`, `WorkbookView` |
| `src/lib/grid/commands.ts`, `keyboard.ts`, `clipboard.ts`, `pointer.ts` | Every grid edit goes through `Session.edit`; the keyboard map; copy and paste; mouse selection |
| `src/routes/` | Workbook list with import Dropzone and EmptyState, `/w/[id]`, `/trash`, `/settings` |

Grid scope covered (plan 7.7): scroll, cell and range selection, typing and in-cell editing, formula bar, sheet tabs (add, rename, delete, move left and right), undo and redo, copy and paste (internal with styles and formulas, plain text from other apps), bold, italic, underline, text colour, fill colour, horizontal alignment, number formats (automatic, number, percent, currency, date), column and row resize, insert and delete rows and columns, freeze panes, and other people's cursors.

Not covered in v1: picking a reference with the mouse while typing a formula, formula autocompletion and highlighting, the link tooltip, drag-to-reorder tabs (the tab menu moves them instead), unhiding hidden sheets, merge and border tools, editing on a phone (the phone gets the read view), and translations (the UI is English).

## Tests

- `protocol.test.ts`: golden byte vectors for every frame, HELLO and INIT's `acked` included, written from the `docs/api.md` table, plus malformed frames.
- `session.test.ts`: the session against a fake socket and a fake room, one test per path (HELLO, echo, reject, replay drop, reconnect and resend, a lost echo never resent, numbering after a reload, close codes 1008, 1009, 1012, 1013 and 1001, sequence gap, viewer, presence, cursor throttle, keepalive).
- `convergence.test.ts`: 1000 random cases (set `SESSION_CASES` for more). Three sessions, random edits, random delivery order, random dropped connections. At the end every `confirmed` and every `view` equals the server byte for byte, and the fake room, which tracks `acked` per origin like the API, never saw an acknowledged batch again. The test fails within a few seeds if the view rebuild is removed.
- `backend.test.ts`, `keyboard.test.ts`, `clipboard.test.ts`, `commands.test.ts`, and small pure helpers.

Checked by hand with two headless Chromium pages against the mock: one types `1`, `2`, `3` and `=SUM(A1:A3)` in A1 to A4 while the other sets A2 to `10`; both show `14` within a second, and a bold set in one page shows in the other.

## Contract notes

`docs/api.md` at the repo root is the contract with the API. The client follows it, including:

1. **Session route.** `GET /api/auth/session` answers `{"user": {"id","email","name","avatar_url"?} | null}`. Sign-in goes to `GET /api/auth/oidc`, sign-out is `POST /api/auth/logout`.
2. **Engine version.** The client reads `engine_version` from the workbook when present, else `engine` from `GET /version`, and compares the `x.y.z` part with `@ironcalc/wasm`'s version. Neither present means the workbook is refused.
3. **HELLO and resends.** Every socket sends HELLO first with a 16-byte origin. `claimOrigin` (`src/lib/engine/origin.ts`) keeps it per workbook in `sessionStorage` and holds it with a Web Lock, so a duplicated tab takes a fresh one instead of sharing it. On every INIT the replica drops the pending batches with `client_seq <= acked`, resends the rest under their own numbers, and numbers new batches above `acked`. A batch the server accepted whose echo was lost is never sent again (`session.test.ts`, and `convergence.test.ts` checks it on every case).
4. **Close codes.** 1008 stops: reason `unauthenticated` shows "sign in again", anything else "no access". 1012 reconnects at once. 1001, 1013, 1011 and network drops reconnect with backoff (0.5 to 10 seconds). 1009 drops the first batch the server had not accepted and tells the user.
5. **Keepalive.** A browser cannot send WebSocket pings, so the client resends its CURSOR frame every 25 seconds. The server counts any client frame as activity.
6. **Refused upgrades.** The server completes a refused upgrade and closes it with 1008, so the client stops. The page loads `GET /api/workbooks/{id}` first; a 404 shows the no-access screen.
7. **Oversized batches.** The client refuses a batch over 1 MiB locally and rebuilds the view, so 1009 should not happen in practice.

## Deviations

- The vendored files live in `src/lib/grid/canvas/vendor/` rather than directly in `src/lib/grid/canvas/`. The root `filet.yml` already ignores any `vendor` directory, and the client agent does not own that file.
- `Editor/util.tsx` is not vendored: it is a React module. The renderer took only `getColor` from it, now `src/lib/grid/canvas/palette.ts` over muse's `USER_COLORS`.
- Toolbar glyphs muse does not ship (bold, italic, align, undo and others) are Solar icons bundled in `src/lib/icons/icons-data.json` and registered with `registerIcons`, as muse's `icons.ts` asks. Solar by 480 Design, CC BY 4.0, taken from `@iconify-json/solar` 1.2.14.
