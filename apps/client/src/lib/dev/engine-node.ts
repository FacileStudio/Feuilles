import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { initSync, Model } from '@ironcalc/wasm';

let loaded = false;

/** Loads the WASM engine synchronously under Node, for tests. Never imported by the app. */
export function loadEngineSync(): void {
	if (loaded) return;
	const require = createRequire(import.meta.url);
	initSync({ module: readFileSync(require.resolve('@ironcalc/wasm/wasm_bg.wasm')) });
	loaded = true;
}

/** Opens a snapshot in English. */
export const load = (bytes: Uint8Array): Model => Model.fromBytes(bytes, 'en');

/** A fresh one-sheet workbook snapshot. */
export function emptySnapshot(): Uint8Array {
	const model = new Model('Workbook', 'en', 'UTC', 'en');
	const bytes = model.toBytes();
	model.free();
	return bytes;
}
