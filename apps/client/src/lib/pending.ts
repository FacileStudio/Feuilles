import type { PendingAction } from './api/types';

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
