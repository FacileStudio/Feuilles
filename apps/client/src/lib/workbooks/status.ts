import type { ConnectionState } from '@facile/muse';
import type { SessionStatus } from '$lib/engine/session.svelte';

const states: Record<SessionStatus, ConnectionState> = {
	open: 'live',
	connecting: 'connecting',
	reconnecting: 'offline',
	revoked: 'stopped',
	closed: 'stopped'
};

/** The muse `ConnectionHint` state of an open workbook's socket. */
export const connectionState = (status: SessionStatus): ConnectionState => states[status];

const notices: Record<string, string> = {
	rate_limited: 'You are editing faster than the server takes edits. The last one was undone; try again.',
	permission_denied: 'You can only view this workbook. The edit was undone.',
	too_large: 'That change is too large to send at once. Try a smaller range.'
};

/** Words for a refused edit, from the REJECT reason. */
export function noticeText(reason: string): string {
	return notices[reason] ?? 'The server refused an edit, so it was undone.';
}
