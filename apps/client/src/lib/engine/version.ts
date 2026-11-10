import manifest from '@ironcalc/wasm/package.json';

/** Version of the bundled `@ironcalc/wasm`, which must equal the server's `ironcalc` crate. */
export const ENGINE_VERSION: string = manifest.version;

const semver = /\d+\.\d+\.\d+/;

/**
 * Whether a workbook written by engine `stored` is safe to open with the bundled engine `loaded`.
 * Only the `x.y.z` part counts, so `ironcalc 0.9.1` and `0.9.1` match; anything else, a missing
 * version included, is refused: diffs are bitcode of private types and a mismatch corrupts silently.
 */
export function engineMatches(stored: string | null | undefined, loaded: string): boolean {
	const left = stored?.match(semver)?.[0];
	return left !== undefined && left === loaded.match(semver)?.[0];
}
