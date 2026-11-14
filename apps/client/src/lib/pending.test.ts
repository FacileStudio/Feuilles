import { describe, expect, it } from 'vitest';
import { targetArgs, targetWorkbook } from './pending';

describe('pending', () => {
	it('reads the workbook of a stored target, and nothing from a malformed one', () => {
		const target = JSON.stringify({ workbook_id: 'w1', sheet: 0, op: { op: 'delete_sheet' } });
		expect(targetWorkbook({ target })).toBe('w1');
		expect(targetWorkbook({ target: 'not json' })).toBeNull();
		expect(targetArgs({ target: 'not json' })).toBe('not json');
	});
});
