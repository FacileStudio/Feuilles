import { describe, expect, it } from 'vitest';
import { noticeText, statusLabel } from './status';

describe('workbook status', () => {
	it('labels each session status', () => {
		expect(statusLabel('open')).toEqual({ tone: 'success', label: 'Live' });
		expect(statusLabel('reconnecting').tone).toBe('warning');
		expect(statusLabel('revoked').tone).toBe('danger');
	});

	it('explains known and unknown refusals', () => {
		expect(noticeText('rate_limited')).toContain('faster');
		expect(noticeText('Invalid sheet index')).toBe('The server refused an edit, so it was undone.');
	});
});
