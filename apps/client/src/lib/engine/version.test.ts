import { describe, expect, it } from 'vitest';
import { engineMatches } from './version';

describe('engineMatches', () => {
	it('accepts the same version, with or without a prefix', () => {
		expect(engineMatches('0.9.1', '0.9.1')).toBe(true);
		expect(engineMatches('ironcalc 0.9.1', '0.9.1')).toBe(true);
	});

	it('refuses another version or none', () => {
		expect(engineMatches('0.9.0', '0.9.1')).toBe(false);
		expect(engineMatches('0.10.1', '0.9.1')).toBe(false);
		expect(engineMatches('', '0.9.1')).toBe(false);
		expect(engineMatches(undefined, '0.9.1')).toBe(false);
	});
});
