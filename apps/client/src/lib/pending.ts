import { ApiError } from './api/error';
import type { PendingAction } from './api/types';

/** tronc's refusal of anything but the person's own browser session on the confirm page (`tronc::mcp::DELEGATED`). */
export const OWN_SESSION_REFUSAL = "only the person's own session can confirm actions";

/**
 * Whether `error` is the confirm page's refusal of a session that is not the person's own browser login: a
 * cookie from before the upgrade, a CLI or API token, or the mobile or desktop app. Signing in again in the
 * browser fixes it; any other 403, such as a demotion to viewer, does not.
 */
export function notOwnSession(error: unknown): boolean {
	return (
		error instanceof ApiError &&
		error.status === 403 &&
		error.code === 'permission_denied' &&
		error.message === OWN_SESSION_REFUSAL
	);
}

/** How the confirm page describes an action's state: ready to run, or its workbook changed or vanished since. */
export const STATES: Record<PendingAction['state'], { tone: 'success' | 'warning' | 'danger'; label: string }> = {
	ready: { tone: 'success', label: 'Ready' },
	changed: { tone: 'warning', label: 'The workbook changed since the request: confirming will fail' },
	missing: { tone: 'danger', label: 'The workbook is gone, or you can no longer edit it' }
};

/** The parsed target of an action: the agent's arguments, or the raw text when it is not JSON. */
export function targetArgs(action: Pick<PendingAction, 'target'>): Record<string, unknown> | string {
	try {
		return JSON.parse(action.target) as Record<string, unknown>;
	} catch {
		return action.target;
	}
}

/** The workbook an action touches, or null. */
export function targetWorkbook(action: Pick<PendingAction, 'target'>): string | null {
	const args = targetArgs(action);
	return typeof args === 'object' && args !== null && typeof args.workbook_id === 'string' ? args.workbook_id : null;
}
