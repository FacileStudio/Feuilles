import init, { Model } from '@ironcalc/wasm';
import wasmUrl from '@ironcalc/wasm/wasm_bg.wasm?url';

export { Model };

export { ENGINE_VERSION } from './version';

let ready: Promise<void> | null = null;

/** Loads the WASM engine once. Every later call returns the same promise. */
export function loadEngine(): Promise<void> {
	ready ??= init({ module_or_path: wasmUrl }).then(() => undefined);
	return ready;
}

/** Opens a snapshot in the workbook's display language. */
export function modelFromBytes(bytes: Uint8Array, language: string): Model {
	return Model.fromBytes(bytes, language);
}
