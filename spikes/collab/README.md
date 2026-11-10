# F0 collaboration spike

This crate checks that the collaboration design in plan section 7.1 works on IronCalc 0.9.1 (`ironcalc = "=0.9.1"` from crates.io, `@ironcalc/wasm@0.9.1` from npm). It is throwaway code. Nothing here ships.

## Answers

| # | Question | Answer |
|---|---|---|
| 1 | Do 3 clients and a server following 7.1 converge? | Yes. 10 000 random cases pass: every `confirmed` model and the server have equal `to_bytes()`, and every `view` equals its `confirmed` whenever it has no pending batch. |
| 2 | Is `UserModel<'static>` `Send`? | Yes, and `Sync` too. It compiles as a const assertion, and a model moves to another thread. |
| 3 | Do diffs cross between WASM and native? | Yes, both directions. Same cells, and the bytes are identical too: `to_bytes()` after the replay matches byte for byte. This holds on bun and on Node 20. |
| 4 | How fast and how big are snapshots? | About 15 bytes per filled cell. 1M cells is 15.2 MB, with 43 ms `to_bytes` and 58 ms `from_bytes` native, and 56 to 94 ms in WASM. Full table below. |

Two findings change how F2 must build the server. Neither one breaks the design.

## Finding A: a rejected batch can change the model

Plan 7.1 says that on error the server rejects the batch "and changes nothing". IronCalc does not do that for you: `apply_external_diffs` is not atomic.

Smallest case (`tests/reject.rs`): a client adds sheet 2 and then styles a cell on it. The server rejected the sheet, so the style batch arrives on a workbook with one sheet. `apply_external_diffs` returns `Err("Invalid sheet index")`, but the server's `to_bytes()` has already changed.

The cause is in `ironcalc_base-0.9.1/src/styles.rs`, `Model::set_cell_style`. It calls `self.workbook.styles.get_style_index_or_create(style)`, which appends the new style to the workbook, before `self.workbook.worksheet_mut(sheet)?` finds out the sheet is missing. `set_column_style` and `set_row_style` in `model.rs` follow the same order. A batch with several diffs can also fail halfway, because `apply_external_diffs` applies its diffs one by one and stops at the first error.

In the 10 000 cases, 32 of the 3 497 rejects had already changed the server model. If the server keeps that model, its bytes drift away from every client's `confirmed` model. The simulation's server avoids that by taking `to_bytes()` before each apply and loading that copy back when the apply fails.

What F2 needs to decide:
- Snapshot before every apply. Simple, but it costs one `to_bytes` per PUSH: 2.5 ms at 100k cells and 43 ms at 1M.
- Or, on a reject, drop the room and reload it from the database (snapshot plus `workbook_updates`). This costs nothing on the accepted path. A reject is rare, and the 7.3 load path already exists. This is the cheaper option.

Rejects also cascade. Once a client's batch is rejected, its later pending batches that relied on it (here, a style on the refused sheet) are rejected too. The server stays consistent, and the client hears about each refusal.

## Finding B: volatile functions converge on formulas, not on values

`RAND()` and `RANDBETWEEN()` are recalculated on each replica, so their cached values differ between the server and each client, and so does `to_bytes()`. The formula text converges. `NOW()` and `TODAY()` matched in this test only because both replicas ran within the same millisecond. `tests/volatile.rs` pins this behaviour.

What this means for F2 and later:
- Equal `to_bytes()` is a valid test invariant only for workbooks without volatile functions. The property test leaves them out on purpose.
- A snapshot saves the server's random value. Each browser shows its own value until the next recalculation.

## Other observations

- In the 10 000 cases, a pending batch failed 18 880 times while it was replayed onto a rebuilt `view`. One plausible cause, not traced case by case, is a remote edit that took the name of a sheet the batch creates or renames. The plan does not say what to do then. The simulation ignores the error, and the server later rejects that same batch, which rebuilds `view` correctly. So `view` is briefly missing an edit that is about to be refused. That is acceptable, but `Session` must not treat a replay error as fatal.
- A local edit that IronCalc refuses (584 cases; I did not classify them) puts nothing in the send queue. `Session` must not send an empty batch. The spike compares the batch with the bytes of an empty flush.
- `from_bytes` does not re-evaluate the workbook. The snapshot already holds the cached values, so INIT needs no `evaluate()`.
- `Workbook.tables` and `Worksheet.links` are `HashMap`s. bitcode writes them in hash order, so two equal workbooks with two or more hyperlinks may not give equal bytes. The spike does not cover links. Any F2 test that compares bytes must avoid them, or compare cells instead.

## Question 1: property test

`tests/convergence.rs`, using the code in `src/`:
- `Server` applies batches in arrival order, counts `seq`, broadcasts every accepted batch to all clients including the sender, and rejects a failed batch to the sender only (with the rollback from finding A).
- `Client` follows the 7.1 pseudo-code line by line. It keeps `confirmed`, `view` and `pending`, rebuilds `view` with `from_bytes(confirmed.to_bytes())` plus the pending batches when a remote batch lands while it has a local one in flight, and rebuilds on REJECT.
- `Room` puts one FIFO between each client and the server in each direction. A case is 1 to 59 random steps: a local edit on client *n*, the server taking the next PUSH from client *n*, or client *n* taking its next frame. The order of these steps across clients is random. After the last step the room delivers every frame still in flight.
- The random edits: numbers, text, empty input, formulas (including cross-sheet references, a broken formula, and references to sheets that may not exist), seven style paths on small areas and on whole columns, row and column inserts and deletes, new sheets, and renames to colliding names.
- The checks: after each step, every client with nothing pending has `view` bytes equal to its `confirmed` bytes. At the end, every `confirmed` model equals the server byte for byte, every client has seen every `seq`, and the server snapshot survives a `from_bytes`/`to_bytes` round trip.

The 10 000 cases took 137 s in release mode. They ran 142 509 accepted batches, 3 497 rejects, 202 382 view rebuilds and 584 refused local edits. The test also asserts that the reject and rebuild paths ran at least once.

I checked that the test can fail: a deliberate bug that overwrote one client's `confirmed` with its `view` failed it within 200 cases.

## Question 3: cross-platform

`tests/cross_platform.rs` drives `js/xplat.mjs`, which loads the npm package with `initSync`. Its `web` target runs in bun and Node as it is, so I did not rebuild it with wasm-pack.

1. Native writes an empty base snapshot, makes 14 edits (cells, formulas, unicode text, fill and number format, column insert, row delete, new sheet, rename to `Résumé`, row insert), and writes the batch, a dump of every cell's content and formatted value, and its `to_bytes()`.
2. WASM applies the native batch on the base. Its cell dump and its `toBytes()` both match native's. It also loads the native snapshot and gets the same cells. It then makes its own 15 edits (including rename to `Données` and a cross-sheet formula) and writes the same three files.
3. Native applies the WASM batch. The cell dump and `to_bytes()` match. It also loads the WASM snapshot and gets the same cells.

The bench (question 4) adds one more check: `toBytes()` in WASM of a snapshot built natively gives identical bytes at 1k, 100k and 1M cells.

## Question 4: snapshot timings and sizes

Workbook: one sheet, 100 columns, filled row by row. A third of the cells are numbers, a third are short strings, and a third are formulas (`=A{row}+1`), evaluated before `to_bytes`. The table shows medians over 51, 11 and 5 runs, on an AMD Ryzen 5 3600 (12 threads). Native is a release build. WASM is `@ironcalc/wasm` 0.9.1 under bun 1.4.2 and Node 20, and its timings include copying the bytes across the JS boundary.

| Filled cells | Bytes | Native `to_bytes` | Native `from_bytes` | bun `toBytes` | bun `fromBytes` | Node `toBytes` | Node `fromBytes` |
|---|---|---|---|---|---|---|---|
| 1 000 | 14 572 | 0.03 ms | 0.08 ms | 0.15 ms | 0.31 ms | 0.18 ms | 0.41 ms |
| 100 000 | 1 486 740 | 2.5 ms | 4.6 ms | 4.1 ms | 3.9 ms | 6.8 ms | 9.0 ms |
| 1 000 000 | 15 200 290 | 43 ms | 58 ms | 94 ms | 57 ms | 56 ms | 47 ms |

The WASM numbers vary by about 2x between runs, and native `from_bytes` is not always faster than WASM `fromBytes`. Read the table as orders of magnitude. A 1M-cell INIT frame is about 15 MB and loads in well under 100 ms, so the 1 MiB PUSH cap is safe, and snapshots never need to be chunked for speed.

## Running it

```sh
cd js && bun install && cd ..
cargo test                                   # every question; set COLLAB_CASES for fewer cases
cargo test --release --test convergence -- --nocapture   # 10 000 cases with totals
COLLAB_JS=node cargo test --test cross_platform           # question 3 on Node instead of bun
cargo run --release --bin bench -- target/bench && bun js/bench.mjs target/bench
```
