import type { Tone } from '@facile/muse';
import type { SessionStatus } from '$lib/engine/session.svelte';

/** The status dot of an open workbook. */
export function statusLabel(status: SessionStatus): { tone: Tone; label: string } {
	if (status === 'open') return { tone: 'success', label: 'Live' };
	if (status === 'revoked') return { tone: 'danger', label: 'No access' };
	if (status === 'closed') return { tone: 'neutral', label: 'Closed' };
	return { tone: 'warning', label: status === 'connecting' ? 'Connecting' : 'Reconnecting' };
}

const notices: Record<string, string> = {
	rate_limited: 'You are editing faster than the server takes edits. The last one was undone; try again.',
	permission_denied: 'You can only view this workbook. The edit was undone.',
	too_large: 'That change is too large to send at once. Try a smaller range.'
};

/** Words for a refused edit, from the REJECT reason. */
export function noticeText(reason: string): string {
	return notices[reason] ?? 'The server refused an edit, so it was undone.';
}
