import { describe, expect, it } from 'vitest';
import { rememberReturn, safeReturn, takeReturn } from './return';

function memory(): Storage {
	const items = new Map<string, string>();
	return {
		get length() {
			return items.size;
		},
		clear: () => items.clear(),
		getItem: (key) => items.get(key) ?? null,
		key: (index) => [...items.keys()][index] ?? null,
		removeItem: (key) => void items.delete(key),
		setItem: (key, value) => void items.set(key, value)
	};
}

describe('return after sign-in', () => {
	it('keeps a local path once and refuses anything that leaves the origin', () => {
		const storage = memory();
		rememberReturn('/pending?action=a1', storage);
		expect(takeReturn(storage)).toBe('/pending?action=a1');
		expect(takeReturn(storage)).toBeNull();
		for (const bad of ['https://evil.test', '//evil.test', '/\\evil.test', '', null]) expect(safeReturn(bad)).toBeNull();
	});
});
