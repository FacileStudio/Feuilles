import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import { initSync, Model } from "@ironcalc/wasm";

const require = createRequire(import.meta.url);
initSync({ module: readFileSync(require.resolve("@ironcalc/wasm/wasm_bg.wasm")) });

const dir = process.argv[2];
const read = (name) => new Uint8Array(readFileSync(join(dir, name)));

const window = Array.from({ length: 144 }, (_, i) => [Math.floor(i / 12) + 1, (i % 12) + 1]);

function cellLine(model, sheet, row, column) {
  const content = model.getCellContent(sheet, row, column);
  if (content === "") return [];
  return [`${sheet}\t${row}\t${column}\t${content}\t${model.getFormattedCellValue(sheet, row, column)}`];
}

function dumpCells(model) {
  return model
    .getWorksheetsProperties()
    .flatMap((sheet, index) => [
      `sheet\t${index}\t${sheet.name}`,
      ...window.flatMap(([row, column]) => cellLine(model, index, row, column)),
    ])
    .join("\n");
}

function sameBytes(a, b) {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

const failures = [];
const base = read("base.bin");

const fromNative = Model.fromBytes(base, "en");
fromNative.applyExternalDiffs(read("native_diff.bin"));
if (dumpCells(fromNative) !== readFileSync(join(dir, "native_cells.txt"), "utf8")) {
  failures.push("native diff applied on wasm gives different cells");
}
if (!sameBytes(fromNative.toBytes(), read("native_after.bin"))) {
  failures.push("native diff applied on wasm gives different toBytes");
}
const nativeSnapshot = Model.fromBytes(read("native_after.bin"), "en");
if (dumpCells(nativeSnapshot) !== readFileSync(join(dir, "native_cells.txt"), "utf8")) {
  failures.push("native snapshot loaded in wasm gives different cells");
}

const local = Model.fromBytes(base, "en");
local.setUserInput(0, 1, 1, "7");
local.setUserInput(0, 2, 1, "35");
local.setUserInput(0, 3, 1, "=A1*A2");
local.setUserInput(0, 4, 1, "naïve 😀 text");
local.setUserInput(0, 5, 2, "=SUM(A1:A3)/7");
local.setUserInput(0, 6, 2, "3.14159");
local.updateRangeStyle({ sheet: 0, row: 1, column: 1, width: 2, height: 3 }, "font.b", "true");
local.updateRangeStyle({ sheet: 0, row: 6, column: 2, width: 1, height: 1 }, "num_fmt", "0.00");
local.insertRows(0, 2, 1);
local.deleteColumns(0, 3, 1);
local.newSheet();
local.renameSheet(1, "Données");
local.setUserInput(1, 1, 1, "=Sheet1!A4&\"!\"");
local.insertColumns(0, 1, 2);
local.setUserInput(0, 9, 9, "=LEN(A5)");
const wasmDiff = local.flushSendQueue();
writeFileSync(join(dir, "wasm_diff.bin"), wasmDiff);
writeFileSync(join(dir, "wasm_cells.txt"), dumpCells(local));
writeFileSync(join(dir, "wasm_after.bin"), local.toBytes());

const replay = Model.fromBytes(base, "en");
replay.applyExternalDiffs(wasmDiff);
if (!sameBytes(replay.toBytes(), local.toBytes())) {
  failures.push("wasm diff replayed on wasm gives different toBytes");
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log("wasm side ok");
