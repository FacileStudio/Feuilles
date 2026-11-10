import { describe, expect, it } from 'vitest';
import { claimOrigin, type OriginLocks, type OriginStorage } from './origin';

function memoryStorage(): OriginStorage & { items: Map<string, string> } {
	const items = new Map<string, string>();
	return { items, getItem: (key) => items.get(key) ?? null, setItem: (key, value) => void items.set(key, value) };
}

function memoryLocks(): OriginLocks {
	const held = new Set<string>();
	const request = (async (name: string, _options: LockOptions, run: (lock: Lock | null) => unknown) => {
		if (held.has(name)) return run(null);
		held.add(name);
		await run({ name, mode: 'exclusive' } as Lock);
		held.delete(name);
	}) as unknown as LockManager['request'];
	return { request };
}

describe('claimOrigin', () => {
	it('keeps one origin per workbook in session storage across claims', async () => {
		const storage = memoryStorage();
		const locks = memoryLocks();
		const first = await claimOrigin('w1', storage, locks);
		expect(first.origin).toHaveLength(16);
		first.release();
		await Promise.resolve();
		const again = await claimOrigin('w1', storage, locks);
		expect(Array.from(again.origin)).toEqual(Array.from(first.origin));
		const other = await claimOrigin('w2', storage, locks);
		expect(Array.from(other.origin)).not.toEqual(Array.from(first.origin));
	});

	it('takes a fresh origin when another tab holds the stored one', async () => {
		const storage = memoryStorage();
		const locks = memoryLocks();
		const first = await claimOrigin('w1', storage, locks);
		const copy = memoryStorage();
		copy.items.set('feuilles.origin.w1', storage.items.get('feuilles.origin.w1')!);
		const duplicate = await claimOrigin('w1', copy, locks);
		expect(Array.from(duplicate.origin)).not.toEqual(Array.from(first.origin));
		expect(copy.items.get('feuilles.origin.w1')).not.toBe(storage.items.get('feuilles.origin.w1'));
	});

	it('works without storage or locks', async () => {
		const claim = await claimOrigin('w1', undefined, undefined);
		expect(claim.origin).toHaveLength(16);
	});

	it('ignores a stored value that is not 16 hex bytes', async () => {
		const storage = memoryStorage();
		storage.items.set('feuilles.origin.w1', 'not hex');
		const claim = await claimOrigin('w1', storage, memoryLocks());
		expect(storage.items.get('feuilles.origin.w1')).toMatch(/^[0-9a-f]{32}$/);
		expect(claim.origin).toHaveLength(16);
	});
});
