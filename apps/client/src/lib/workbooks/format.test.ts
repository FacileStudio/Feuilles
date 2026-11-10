import { describe, expect, it } from 'vitest';
import { ago, cleanTitle, titleFromFile } from './format';

describe('workbook formatting', () => {
	it('says how long ago a date was', () => {
		const now = Date.parse('2026-11-10T12:00:00Z');
		expect(ago('2026-11-10T11:57:00Z', now)).toBe('3 minutes ago');
		expect(ago('2026-11-09T12:00:00Z', now)).toBe('yesterday');
		expect(ago('2026-11-10T11:59:59Z', now)).toBe('just now');
	});

	it('enforces the 1 to 200 character title rule after trimming', () => {
		expect(cleanTitle('  Budget  ')).toBe('Budget');
		expect(cleanTitle('   ')).toBeNull();
		expect(cleanTitle('x'.repeat(201))).toBeNull();
		expect(cleanTitle('é'.repeat(200))).toBe('é'.repeat(200));
	});

	it('names an import after its file', () => {
		expect(titleFromFile('Q3 budget.xlsx')).toBe('Q3 budget');
		expect(titleFromFile('.xlsx')).toBe('Imported workbook');
	});
});
