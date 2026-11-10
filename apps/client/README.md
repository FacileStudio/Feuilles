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
| `src/lib/engine/protocol.ts`, `protocol-server.ts`, `wire.ts` | The frames of plan 7.2, byte for byte. The server-side encoders serve the tests and the mock |
| `src/lib/engine/replica.ts` | The 7.1 algorithm: `confirmed`, `view`, `pending`, the reject path, and the F0 notes (no empty batch; a pending batch that no longer replays is dropped) |
| `src/lib/engine/session.svelte.ts` | Socket, INIT, reconnect with backoff and resend under new `client_seq` values, presence, cursor frames at most 10 per second |
| `src/lib/engine/version.ts` | The engine version guard |
| `src/lib/grid/canvas/` | IronCalc v0.9.1's `WorksheetCanvas` vendored under `vendor/`, see `NOTICE` |
| `src/lib/grid/*.svelte` | `Grid`, `CellEditor`, `FormulaBar`, `Toolbar`, `SheetTabs`, `ContextMenu`, `Cursors`, `WorkbookView` |
| `src/lib/grid/commands.ts`, `keyboard.ts`, `clipboard.ts`, `pointer.ts` | Every grid edit goes through `Session.edit`; the keyboard map; copy and paste; mouse selection |
| `src/routes/` | Workbook list with import Dropzone and EmptyState, `/w/[id]`, `/trash`, `/settings` |

Grid scope covered (plan 7.7): scroll, cell and range selection, typing and in-cell editing, formula bar, sheet tabs (add, rename, delete, move left and right), undo and redo, copy and paste (internal with styles and formulas, plain text from other apps), bold, italic, underline, text colour, fill colour, horizontal alignment, number formats (automatic, number, percent, currency, date), column and row resize, insert and delete rows and columns, freeze panes, and other people's cursors.

Not covered in v1: picking a reference with the mouse while typing a formula, formula autocompletion and highlighting, the link tooltip, drag-to-reorder tabs (the tab menu moves them instead), unhiding hidden sheets, merge and border tools, editing on a phone (the phone gets the read view), and translations (the UI is English).

## Tests

- `protocol.test.ts`: golden byte vectors for every frame, written from the 7.2 table, plus malformed frames.
- `session.test.ts`: the session against a fake socket and a fake room, one test per path (echo, reject, replay drop, reconnect and resend, sequence gap, 1008, viewer, presence, cursor throttle, keepalive).
- `convergence.test.ts`: 1000 random cases (set `SESSION_CASES` for more). Three sessions, random edits, random delivery order, random dropped connections. At the end every `confirmed` and every `view` equals the server byte for byte. The test fails within a few seeds if the view rebuild is removed.
- `backend.test.ts`, `keyboard.test.ts`, `clipboard.test.ts`, `commands.test.ts`, and small pure helpers.

Checked by hand with two headless Chromium pages against the mock: one types `1`, `2`, `3` and `=SUM(A1:A3)` in A1 to A4 while the other sets A2 to `10`; both show `14` within a second, and a bold set in one page shows in the other.

## Contract notes and gaps

The client follows plan sections 7.2, 7.5 and 7.7. Where the plan is silent:

1. **Session route.** The plan names `/auth/session` but not its body. The client expects Capsule's: `GET /api/auth/session` answers `{"user": {"email","name"?,"avatar_url"?} | null}`. Sign-in goes to `GET /api/auth/oidc`, sign-out is `POST /api/auth/logout`.
2. **Engine version.** The `Workbook` struct in 7.5 has no `engine_version`, while 7.7 says the client compares it. The client reads `engine_version` from the workbook when present, else `engine` from `GET /version`, and compares the `x.y.z` part with `@ironcalc/wasm`'s version. Neither present means the workbook is refused. `/version` sits outside `/api`, so the dev proxy forwards it too.
3. **`SheetInfo`.** Its fields are not given. The client reads `name` only.
4. **Keepalive.** 7.2 says the client pings every 25 seconds, but a browser cannot send WebSocket ping frames. The client resends its CURSOR frame every 25 seconds instead. The server must count any client frame as activity.
5. **Resend can duplicate.** On reconnect the client resends every pending batch, as 7.2 says. A batch the server accepted just before the socket died, whose echo was lost, is applied twice. Every replica still converges, but an inserted row can appear twice. Closing this needs an idempotency key, for example the server remembering `(subject, connection nonce, client_seq)` across a reconnect.
6. **Refused upgrades.** A 401 or 404 on the WebSocket upgrade reaches the browser as close code 1006, like a network drop, so the client keeps retrying with backoff (0.5 to 10 seconds). Only 1008 stops it. The page loads `GET /api/workbooks/{id}` first, which catches most of these.
7. **Oversized batches.** The client refuses a batch over 1 MiB locally and rebuilds the view, rather than letting the server close with 1009 and resending it forever.
8. **Space roles.** `GET /spaces` roles are taken as `member`, `admin` or `owner`.

## Deviations

- The vendored files live in `src/lib/grid/canvas/vendor/` rather than directly in `src/lib/grid/canvas/`. The root `filet.yml` already ignores any `vendor` directory, and the client agent does not own that file.
- `Editor/util.tsx` is not vendored: it is a React module. The renderer took only `getColor` from it, now `src/lib/grid/canvas/palette.ts` over muse's `USER_COLORS`.
- Toolbar glyphs muse does not ship (bold, italic, align, undo and others) are Solar icons bundled in `src/lib/icons/icons-data.json` and registered with `registerIcons`, as muse's `icons.ts` asks. Solar by 480 Design, CC BY 4.0, taken from `@iconify-json/solar` 1.2.14.
