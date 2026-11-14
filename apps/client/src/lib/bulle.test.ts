import { describe, expect, it } from 'vitest';
import { bulleMessage, embedUrl, trustedOrigin } from './bulle';

describe('bulle', () => {
	it('builds the chat embed URL of a workbook on the configured Bulle', () => {
		const page = { app: 'feuilles', id: 'w1', title: 'Budget & co', url: 'https://feuilles.facile.studio/w/w1' } as const;
		const url = new URL(embedUrl(page, 'https://bulle.test'));
		expect(url.origin + url.pathname).toBe('https://bulle.test/embed/chat');
		expect(Object.fromEntries(url.searchParams)).toEqual({ ...page });
	});

	it('trusts the configured Bulle, and localhost only while developing', () => {
		expect(trustedOrigin('https://bulle.test', 'https://bulle.test')).toBe(true);
		expect(trustedOrigin('https://bulle.facile.studio', 'https://bulle.test')).toBe(false);
		expect(trustedOrigin('http://localhost:5173', 'https://bulle.test')).toBe(false);
		expect(trustedOrigin('http://localhost:5173', 'https://bulle.test', true)).toBe(true);
	});

	it('reads only the two messages Bulle sends', () => {
		expect(bulleMessage({ type: 'bulle:close' })).toEqual({ type: 'bulle:close' });
		expect(bulleMessage({ type: 'bulle:ready', assistant: { name: 'Nova' } })?.type).toBe('bulle:ready');
		expect(bulleMessage({ type: 'bulle:ready' })).toBeNull();
		expect(bulleMessage('bulle:close')).toBeNull();
	});
});
