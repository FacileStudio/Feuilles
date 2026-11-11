# Feuilles — Architecture

How the API is put together: the topology, the collaboration rooms, the data model, sign-in and
the limits v1 accepts.

## Topology

```
Browser ──▶ Traefik ──▶ /api binary (:4000) ──┬──▶ /api/*            handlers (axum on tronc + porte)
  │  WASM IronCalc                             ├──▶ /api/workbooks/{id}/ws   collaboration socket
  │                                            ├──▶ /mcp, /docs, /version, /health
  └── same origin                              └──▶ everything else  SPA from CLIENT_DIR
                                                     │
                                       Hub: shard threads, one room per open workbook
                                                     │
                                                Postgres 16
Registre (sso.facile.studio) ──▶ OIDC sign-in, directory snapshot, deletions, MCP tokens
```

One container, one replica. Rooms live in memory, so two replicas would split a room.

## Modules

| Path | What it is |
|---|---|
| `apps/api/src/serve.rs` | Boot: database, migrations, kits, background tasks, router, graceful shutdown |
| `apps/api/src/auth/` | porte wiring: sessions, OIDC, the directory applier, deletions, the spaces `Store` |
| `apps/api/src/engine/` | IronCalc behind a few calls: load, import and export, read ranges, set inputs |
| `apps/api/src/hub/` | Rooms: shards, the batch path, snapshots, presence, the frame codec |
| `apps/api/src/modules/workbooks/` | Routes, the `access()` rules, the row store, the update log |
| `apps/api/src/modules/live/` | The WebSocket route and the socket loop |
| `apps/api/src/mcp/` | The six MCP tools, with porte's `mcpauth::Verifier` as the token check |
| `apps/api/src/reference/` | The `/docs` table |
| `apps/api/src/upgrade.rs` | `engine-export` and `engine-verify` |
| `apps/api/src/import.rs` | `import-xlsx`, through the route's `modules/workbooks/imports.rs` |

## Collaboration

IronCalc gives two primitives: `flush_send_queue()` returns a batch of edits as opaque bytes,
and `apply_external_diffs()` replays one. It resolves no conflict. Feuilles relies on one rule:
every replica applies the same batches in the same order, and the server decides the order.

The `Hub` runs `min(4, cores)` shard threads. A workbook always maps to the same shard, so one
model is only touched by one thread. For each PUSH, in arrival order, the room:

1. drops it when its `(origin, client_seq)` was already accepted;
2. applies it to the authoritative model; on error it sends REJECT to the sender, then drops the
   model and reloads it from the database, because IronCalc 0.9.1 can half-apply a batch it
   refuses (F0 finding A, `spikes/collab/tests/reject.rs`);
3. inserts the batch into `workbook_updates`, bumps `workbooks.seq` and the origin's ack in one
   transaction; if that fails the room is dropped and every socket closed with 1011;
4. broadcasts UPDATE to every socket, sender included.

The database is the truth; memory is a cache. Every 256 batches, when the last socket leaves,
after 60 seconds empty and on shutdown, the room writes `snapshot = to_bytes()` and deletes the
batches it now includes. Loading a room reads the snapshot and replays the batches after it.

The browser keeps two models: `confirmed`, which applies only server batches in `seq` order, and
`view`, which is `confirmed` plus the pending local batches. Every `confirmed` equals the server
model. `HELLO` and `acked` make resends idempotent across reconnects: see [API](api.md).

API and MCP writes use the same path as client 0, so collaborators see them live.

## Data model

| Table | Key | Holds |
|---|---|---|
| `porte_*` | | porte's accounts, identities and sessions (`migrations/0001_porte.sql`, verbatim) |
| `spaces` | `id` | Registre's spaces, mirrored; `deleted_at` is set when Registre stops listing one |
| `space_members` | `(space_id, subject)` | Members by Registre subject, with name, picture and role |
| `directory_state`, `deletion_state` | `id = 1` | The last applied snapshot version, the deletion cursor |
| `workbooks` | `id` | Title, settings, `engine_version`, `snapshot`, `snapshot_seq`, `seq`, `trashed_at` |
| `workbook_updates` | `(workbook_id, seq)` | Batches after the snapshot, with author, origin and `client_seq` |
| `workbook_acks` | `(workbook_id, origin)` | The highest accepted `client_seq` per browser session; survives snapshots |
| `mcp_actions` | `id` | tronc's MCP confirmation store, unused while no tool needs a confirmation |

A workbook belongs to a space (`space_id`) or to one person (`space_id IS NULL`, `owner_id`).

## Authentication and access

Sign-in is Registre OIDC only (`SSO_ONLY` is forced). porte's middleware reads the cookie, then
`Authorization: Bearer`. Space membership is keyed by Registre subject: a session's account is
mapped to its subject through `porte_identities` under `OIDC_ISSUER`, then
`porte::spaces::Guard` answers the role from `space_members` (`auth/members.rs`, which passes
porte's conformance suite). `modules/workbooks/service.rs::require` is the one gate every route,
socket and tool goes through; no access is 404 and an instance admin gets no bypass.

The directory applier (`auth/spaces.rs`, guards in `auth/retire.rs`) upserts spaces, replaces
members, then closes the sockets of everyone who lost access with 1008. It never deletes a space
or a workbook:

- A space the snapshot no longer lists gets `deleted_at`, loses its member rows (so every
  member loses access at once, and their sockets close with 1008), and its workbooks go to the
  trash (`trashed_at = now()` where it was null). Only the 30-day purge deletes them.
- A space that comes back is undeleted and gets its members again. Its workbooks stay in the
  trash for a manager to restore.
- An empty snapshot while any live space exists is not applied at all: the applier logs it at
  error level and keeps the current state, version included, until a non-empty snapshot comes. A deletion
erases the person's personal workbooks, memberships and account; space workbooks stay with no
owner.

## Known limits of v1

- `RAND()` and `RANDBETWEEN()` compute a different value on every replica. Inputs agree.
- A local batch computed before a remote row or column insert is replayed at its original
  coordinates. No operational transform.
- Rebuilding `view` drops its undo history; it happens only when two people edit within one
  round trip.
- Snapshots are about 15 bytes per filled cell: 1M cells is 15 MB and loads in under 100 ms.
- One API replica. A deploy disconnects every editor for a few seconds; clients reconnect and
  resend.
- A Registre that really drops every space at once is not followed: the last space stays until
  Registre lists at least one space again, or an operator removes it by hand.
