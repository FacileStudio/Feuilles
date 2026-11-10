import { ORIGIN_BYTES } from './protocol';

/** The part of `sessionStorage` an origin claim uses. */
export type OriginStorage = Pick<Storage, 'getItem' | 'setItem'>;

/** The part of the Web Locks API an origin claim uses. */
export type OriginLocks = Pick<LockManager, 'request'>;

/** A HELLO origin this page holds until `release`. */
export type OriginClaim = { origin: Uint8Array; release: () => void };

const toHex = (bytes: Uint8Array) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');

function fromHex(text: string | null): Uint8Array | null {
	if (!text || !/^[0-9a-f]{32}$/.test(text)) return null;
	return Uint8Array.from(text.match(/../g) ?? [], (pair) => parseInt(pair, 16));
}

function read(storage: OriginStorage | undefined, key: string): Uint8Array | null {
	try {
		return fromHex(storage?.getItem(key) ?? null);
	} catch {
		return null;
	}
}

function write(storage: OriginStorage | undefined, key: string, origin: Uint8Array): void {
	try {
		storage?.setItem(key, toHex(origin));
	} catch {
		return;
	}
}

async function hold(locks: OriginLocks, origin: Uint8Array): Promise<(() => void) | null> {
	let release = () => {};
	const held = new Promise<void>((resolve) => (release = resolve));
	const granted = await new Promise<boolean>((answer) => {
		locks
			.request(`feuilles.origin.${toHex(origin)}`, { ifAvailable: true }, (lock) => {
				answer(lock !== null);
				return lock ? held : undefined;
			})
			.catch(() => answer(false));
	});
	return granted ? release : null;
}

/**
 * Claims the HELLO origin of one workbook for this browser tab: the one kept in `sessionStorage`
 * under `scope`, or 16 fresh random bytes. A duplicated tab copies `sessionStorage`, so the origin
 * is also held with a Web Lock: when another tab holds it, this tab takes a fresh one, because two
 * live sockets sharing an origin would have each other's batches dropped as already applied.
 */
export async function claimOrigin(
	scope: string,
	storage: OriginStorage | undefined = globalThis.sessionStorage,
	locks: OriginLocks | undefined = globalThis.navigator?.locks
): Promise<OriginClaim> {
	const key = `feuilles.origin.${scope}`;
	const stored = read(storage, key);
	for (const origin of stored ? [stored, fresh()] : [fresh()]) {
		const release = locks ? await hold(locks, origin) : () => {};
		if (!release) continue;
		write(storage, key, origin);
		return { origin, release };
	}
	const origin = fresh();
	write(storage, key, origin);
	return { origin, release: () => {} };
}

/** 16 random bytes. */
export function fresh(): Uint8Array {
	return crypto.getRandomValues(new Uint8Array(ORIGIN_BYTES));
}
