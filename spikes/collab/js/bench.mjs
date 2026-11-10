import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import { initSync, Model } from "@ironcalc/wasm";

const require = createRequire(import.meta.url);
initSync({ module: readFileSync(require.resolve("@ironcalc/wasm/wasm_bg.wasm")) });

const dir = process.argv[2] ?? "target/bench";
const median = (xs) => xs.sort((a, b) => a - b)[Math.floor(xs.length / 2)];

console.log("cells\tbytes\ttoBytes_ms\tfromBytes_ms\tsame_bytes_as_native");
for (const [cells, runs] of [[1000, 51], [100000, 11], [1000000, 5]]) {
  const native = new Uint8Array(readFileSync(join(dir, `snap-${cells}.bin`)));
  const from = [];
  let model;
  for (let i = 0; i < runs; i++) {
    model?.free();
    const t = performance.now();
    model = Model.fromBytes(native, "en");
    from.push(performance.now() - t);
  }
  const to = [];
  let bytes;
  for (let i = 0; i < runs; i++) {
    const t = performance.now();
    bytes = model.toBytes();
    to.push(performance.now() - t);
  }
  const same = bytes.length === native.length && bytes.every((v, i) => v === native[i]);
  console.log(`${cells}\t${bytes.length}\t${median(to).toFixed(3)}\t${median(from).toFixed(3)}\t${same}`);
  model.free();
  model = undefined;
}
