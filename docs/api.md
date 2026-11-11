# Feuilles — API

Every HTTP route, the collaboration WebSocket protocol, and the MCP tools. The live reference is
`GET /docs` (Scalar) with the OpenAPI 3.1 document at `GET /docs/openapi.json`; a test fails when
a mounted route is missing from it (`apps/api/tests/reference.rs`).

## Conventions

- Routes under `/api` need a session: the `session` cookie (`__Host-session` over HTTPS) or
  `Authorization: Bearer <token>`. A cookie request that is not GET, HEAD or OPTIONS must send
  `X-Facile-CSRF` with any non-empty value, or it gets 403.
- Errors are `{"error":{"code","message"}}`. Codes: `invalid_argument` 400, `unauthenticated` 401,
  `permission_denied` 403, `not_found` 404, `failed_precondition` 412, `resource_exhausted` 413,
  `internal` 500, `unavailable` 503.
- Request bodies refuse unknown fields with 400. JSON bodies are at most 1 MiB, except
  `PUT /cells` (4 MiB) and the import (20 MiB file).
- A workbook the caller cannot see answers 404, never 403, so ids cannot be probed. A path id
  that is not a UUID is 404 too.
- Times are RFC 3339 strings.

## Access

| Caller | Personal workbook | Space workbook |
|---|---|---|
| Owner of the personal workbook | Manager | |
| Space `member` | | Editor |
| Space `admin` or `owner` | | Manager |
| Anyone else, instance admins included | 404 | 404 |

Viewer reads. Editor also edits cells and renames. Manager also trashes, restores and deletes.
A workbook in the trash is read-only for everyone: `PUT /cells` answers 412 and the socket opens
as a viewer.

## Types

`Workbook`:

```json
{
  "id": "uuid", "space_id": "string or null", "title": "string", "locale": "en", "timezone": "UTC",
  "language": "en", "sheets": [SheetInfo], "engine_version": "0.9.1",
  "created_at": "rfc3339", "updated_at": "rfc3339", "trashed_at": "rfc3339 or null"
}
```

`sheets` is filled on single-workbook answers and empty in list answers.
`SheetInfo` is `{"index": 0, "name": "Sheet1", "used_range": "A1:D20" | null}`.
`Cell` is `{"input": "=A1*2", "value": 42.0 | "text" | true | null, "formatted": "42"}`.

## Auth, from porte

| Method and path | Answer |
|---|---|
| `GET /api/auth/config` | `{"sso_only":true,"oidc_enabled":bool}` |
| `GET /api/auth/oidc` | 302 to Registre. Mounted with `OIDC_ISSUER` |
| `GET /api/auth/oidc/callback` | 302 to `OIDC_SUCCESS_URL` with the session cookie |
| `POST /api/auth/oidc/exchange` | `{"code"}` to `{"user_id","token"}`, for a CLI |
| `POST /api/auth/sync-profile` | `{"synced":bool}` |
| `POST /api/auth/backchannel-logout` | form `logout_token`, from Registre |
| `POST /api/auth/logout` | `{"logged_out":true}`, clears the cookie |
| `GET /api/auth/me` | `{"id","email","name","avatar_url"?}`, 401 without a session |
| `GET /api/auth/session` | Always 200: `{"user":{"id","email","name","avatar_url"?}}` or `{"user":null}` |
| `POST /api/registre/sync` | 202. Registre's nudge; wakes the directory sync |
| `POST /api/registre/deletions` | 202. Registre's signed account-deletion push |

## Spaces and workbooks

| Method and path | Body or query | Answer | Access |
|---|---|---|---|
| `GET /api/spaces` | | `{"spaces":[{"id","name","role"}]}` by name | signed in |
| `GET /api/workbooks` | `?space_id=&trashed=false` | `{"workbooks":[Workbook]}`, newest change first | member of `space_id` |
| `POST /api/workbooks` | `{"title","space_id"?,"locale"?,"timezone"?,"language"?}` | 201 `Workbook` | member of `space_id` |
| `POST /api/workbooks/import` | multipart `file` (.xlsx or .csv, at most 20 MiB), `space_id`? | 201 `Workbook`, titled after the file name; a CSV is one sheet of that name | member of `space_id` |
| `GET /api/workbooks/{id}` | | `Workbook` | Viewer |
| `PATCH /api/workbooks/{id}` | `{"title"}` | `Workbook` | Editor |
| `DELETE /api/workbooks/{id}` | | 204, sets `trashed_at` | Manager |
| `POST /api/workbooks/{id}/restore` | | `Workbook` | Manager |
| `DELETE /api/workbooks/{id}/permanent` | | 204; 412 unless in the trash | Manager |
| `GET /api/workbooks/{id}/export.xlsx` | | the file, `Content-Disposition: attachment` | Viewer |
| `GET /api/workbooks/{id}/export.csv` | `?sheet=0` | RFC 4180 CSV of displayed values, CRLF; text starting with `= + - @ ' tab CR` gets a leading `'` | Viewer |
| `GET /api/workbooks/{id}/cells` | `?sheet=0&range=A1:D20` | `{"sheet","range","rows":[[Cell]]}`, at most 10 000 cells | Viewer |
| `PUT /api/workbooks/{id}/cells` | `{"writes":[{"sheet","row","column","input"}]}`, 1 to 5000 | `{"seq"}` | Editor |
| `GET /api/workbooks/{id}/ws` | | WebSocket, below | Viewer |

A missing `space_id` means personal scope; a space the caller is not in is 404. Titles are 1 to
200 characters after trimming. `language` is one of `en`, `es`, `fr`, `de`, `it`. Rows and
columns are 1-based, sheets 0-based. Trash and restore close the workbook's sockets with 1012 so
each reconnects with its new role. Workbooks in the trash for 30 days are deleted by an hourly
task.

`PUT /cells` and the MCP writes go through the workbook's room as client 0: open sockets receive
them as `UPDATE` frames.

### CSV files

An imported CSV becomes one sheet named after the title (`\ / * ? : [ ] '` become spaces, 31
characters at most). The importer reads it this way:

- Encoding: UTF-8, with or without a BOM. Bytes that are not UTF-8 read as Windows-1252, which
  is what Excel's plain "CSV" save writes.
- Delimiter: an Excel `sep=;` first line wins. Otherwise the importer counts `,`, `;` and tab
  outside quotes in the first record and takes the most frequent; a tie goes to `,`. Quoted
  fields may hold delimiters, `""` and line breaks.
- Types: each field is typed like input in a cell (numbers, `12%`, `$5`, dates, `TRUE`, links),
  except that these stay text: a field starting with `=`, digits with a leading zero (`007`,
  `06 12 34 56 78`), `+` and more than six digits (`+33612345678`), and more than 15 digits.
  A `+` or `-` field that the engine would read as a formula (`-A1`, `+SUM(1)`) also stays text.
  No formula is ever imported.
- In a `;` file, a decimal comma (`1,5`, `-0,25`) reads as a number.
- Limits: 20 MiB, 1 048 576 rows, 16 384 columns and 5 000 000 filled cells; past them, 413.

The export writes displayed values, never formulas. A text cell that starts with `=`, `+`, `-`,
`@`, `'`, tab or CR gets a leading `'`, so a spreadsheet opening the file shows it as text
instead of running it (OWASP "CSV injection"). Numbers are never prefixed, so `-5` stays `-5`.
Importing an export gives back the same CSV.

Other routes: `GET /version` answers `{"version":"v0.4.0","engine":"0.9.1"}`; `GET /health` and
`GET /ready` (also under `/api`) answer `{"status":"ok"}` and `{"status":"ready"}`.

## WebSocket protocol

`GET /api/workbooks/{id}/ws`, upgraded. The cookie or a bearer token authenticates. With the
cookie, `Origin` must match the app's own origin, or the upgrade is refused with 403. Binary
frames only; integers are big-endian; the first byte is the frame type.

| Type | Direction | Layout after the type byte |
|---|---|---|
| `0x01` PUSH | client to server | `client_seq: u32`, `payload` (one `flushSendQueue()` batch) |
| `0x02` CURSOR | client to server | UTF-8 JSON `{"sheet":0,"row":1,"column":1,"range":[r1,c1,r2,c2]}` |
| `0x03` HELLO | client to server | `origin: [u8; 16]` |
| `0x10` INIT | server to client | `seq: u64`, `client_id: u32`, `role: u8`, `acked: u32`, `snapshot` |
| `0x11` UPDATE | server to client | `seq: u64`, `client_id: u32`, `client_seq: u32`, `payload` |
| `0x12` REJECT | server to client | `client_seq: u32`, UTF-8 reason |
| `0x13` PRESENCE | server to client | UTF-8 JSON `[{"client_id","name","picture","cursor":{...} or null}]` |

Session flow:

1. The client sends HELLO first. `origin` is 16 random bytes, stable for one browser session
   across its reconnects. Any other first frame closes with 1002; no HELLO within 60 seconds
   closes with 1001.
2. The server answers INIT. `role` is 1 for a viewer, 2 for an editor. `acked` is the highest
   `client_seq` the server accepted from this `origin` on this workbook, 0 when none. `snapshot`
   is `to_bytes()` at `seq`; it needs no `evaluate()`.
3. On every reconnect the client drops the pending batches with `client_seq <= acked` and resends
   the rest. `client_seq` keeps increasing per `origin` across reconnects.
4. Every accepted batch is broadcast as UPDATE to every socket, sender included, in `seq` order.
   `client_id` 0 is the server itself (API and MCP writes; `client_seq` is then 0).
5. A PUSH whose `client_seq` is not above the last accepted one for its `origin` is dropped
   without an answer: it was already applied.

REJECT reasons: `invalid_argument` (empty, a no-op, or the engine refused it; the server then
reloads its model from the database), `permission_denied` (a viewer), `rate_limited` (more than
60 PUSH in a second).

PRESENCE is sent on every join, leave and cursor change, at most 10 times a second. The server
pings every 25 seconds. Any client frame, CURSOR included, counts as activity; after 60 seconds
without one the server closes with 1001. A browser can keep a socket alive by resending CURSOR.

Close codes:

| Code | Reason | Client should |
|---|---|---|
| 1001 | Server restarting, or silence | Reconnect |
| 1002 | Malformed frame, or no HELLO first | Fix the client |
| 1003 | A text frame | Fix the client |
| 1008 | `unauthenticated`, `not_found`, or access removed | Stop retrying |
| 1009 | A PUSH payload over 1 MiB | Drop that batch |
| 1011 | The batch could not be stored | Reconnect and resend |
| 1012 | The workbook went to or out of the trash | Reconnect at once |
| 1013 | More than 8 MiB of broadcasts waiting for this socket | Reconnect |

A refused upgrade (no session, no access, unknown or malformed id) is completed and closed with
1008 and reason `unauthenticated` or `not_found`, because a browser sees an HTTP error on an
upgrade as 1006. A request without upgrade headers gets the plain HTTP 401, 404 or 400.

## MCP

`POST /mcp`, mounted when `MCP_RESOURCE` is set, outside `/api` and the CSRF rule. Streamable
HTTP, stateless (spec 2026-07-28); GET and DELETE answer 405. Tokens come from Registre with
audience `MCP_RESOURCE` and scopes `feuilles.read`, `feuilles.write` (write implies read). The
metadata is at `GET /.well-known/oauth-protected-resource/mcp`. Never plan-gated.

| Tool | Scope | Input | Output |
|---|---|---|---|
| `feuilles_list_workbooks` | read | `{"space_id"?}` | `{"workbooks":[Workbook]}` |
| `feuilles_get_workbook` | read | `{"workbook_id"}` | `Workbook` with sheets and used ranges |
| `feuilles_read_range` | read | `{"workbook_id","sheet"?,"range","cursor"?}` | `{"sheet","range","rows":[[Cell]],"next_cursor"?}`, pages under about 25k tokens |
| `feuilles_write_cells` | write | `{"workbook_id","writes":[{"sheet","row","column","input"}]}` | `{"seq","sheets"}` |
| `feuilles_create_workbook` | write | `{"title","space_id"?}` | `Workbook` |
| `feuilles_add_sheet` | write | `{"workbook_id","name"?}` | `{"seq","sheets"}` |

The access rules above apply to every tool. There is no delete tool.
