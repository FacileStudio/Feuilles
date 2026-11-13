import { readPref, writePref } from '$lib/state/storage';

/** The Bulle origin until the API names one with `bulle_url`. */
export const DEFAULT_BULLE = 'https://bulle.facile.studio';

/** The assistant's name until Bulle answers. */
export const DEFAULT_ASSISTANT = 'Bulle';

/** What Feuilles tells Bulle about the open workbook. */
export interface PageContext {
	app: 'feuilles';
	id: string;
	title: string;
	url: string;
}

/** What the Bulle frame says to Feuilles. */
export type BulleMessage = { type: 'bulle:ready'; assistant: { name: string } } | { type: 'bulle:close' };

/** The chat embed URL for `page` on the Bulle at `origin`. */
export function embedUrl(page: PageContext, origin: string): string {
	const query = new URLSearchParams({ app: page.app, id: page.id, title: page.title, url: page.url });
	return `${origin}/embed/chat?${query}`;
}

/** Whether `origin` may speak for Bulle: the configured Bulle, or a localhost Bulle while developing. */
export function trustedOrigin(origin: string, bulle: string, dev = false): boolean {
	if (origin === bulle) return true;
	return dev && /^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?$/.test(origin);
}

/** The Bulle message in `data`, or null when it is not one. */
export function bulleMessage(data: unknown): BulleMessage | null {
	if (typeof data !== 'object' || data === null) return null;
	const message = data as { type?: unknown; assistant?: { name?: unknown } };
	if (message.type === 'bulle:close') return { type: 'bulle:close' };
	if (message.type === 'bulle:ready' && typeof message.assistant?.name === 'string') {
		return { type: 'bulle:ready', assistant: { name: message.assistant.name } };
	}
	return null;
}

const OPEN_KEY = 'feuilles.bulle.open';

/** Whether `user` left the Bulle sidebar open; false when storage is unavailable. */
export function sidebarOpen(user: string): boolean {
	return readPref(`${OPEN_KEY}.${user}`) === '1';
}

/** Remembers whether `user` has the Bulle sidebar open. */
export function rememberSidebar(user: string, open: boolean): void {
	writePref(`${OPEN_KEY}.${user}`, open ? '1' : '0');
}
