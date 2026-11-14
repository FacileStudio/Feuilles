import { describe, expect, it } from 'vitest';
import { ApiError } from './api/error';
import { notOwnSession, targetArgs, targetWorkbook } from './pending';

describe('pending', () => {
	it('reads the workbook of a stored target, and nothing from a malformed one', () => {
		const target = JSON.stringify({ workbook_id: 'w1', sheet: 0, op: { op: 'delete_sheet' } });
		expect(targetWorkbook({ target })).toBe('w1');
		expect(targetWorkbook({ target: 'not json' })).toBeNull();
		expect(targetArgs({ target: 'not json' })).toBe('not json');
	});

	it('asks to sign in again only for the own-session refusal, not for a lost edit right', async () => {
		const refusal = (message: string) =>
			ApiError.from(
				new Response(JSON.stringify({ error: { code: 'permission_denied', message } }), { status: 403 })
			);
		expect(notOwnSession(await refusal("only the person's own session can confirm actions"))).toBe(true);
		expect(notOwnSession(await refusal('Editor access required'))).toBe(false);
		expect(notOwnSession(new ApiError(401, 'unauthenticated', "only the person's own session can confirm actions"))).toBe(false);
	});
});
