import { describe, expect, it } from 'vitest';
import { connectionState, noticeText } from './status';

describe('workbook status', () => {
	it('maps each session status to a connection state', () => {
		expect(connectionState('open')).toBe('live');
		expect(connectionState('connecting')).toBe('connecting');
		expect(connectionState('reconnecting')).toBe('offline');
		expect(connectionState('revoked')).toBe('stopped');
	});

	it('explains known and unknown refusals', () => {
		expect(noticeText('rate_limited')).toContain('faster');
		expect(noticeText('Invalid sheet index')).toBe('The server refused an edit, so it was undone.');
	});
});
