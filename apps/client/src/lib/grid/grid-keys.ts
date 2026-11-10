import { toast } from '@facile/muse';
import type { Session } from '$lib/engine/session.svelte';
import { LAST_COLUMN, LAST_ROW } from './canvas/vendor/components/WorksheetCanvas/constants';
import type { GridCommands } from './commands';
import type { CellEdit } from './editing.svelte';
import { gridAction, type GridAction } from './keyboard';

/** What a grid key press acts on. */
export type GridContext = { session: Session; commands: GridCommands; edit: CellEdit };

function editing(action: GridAction, { session, commands, edit }: GridContext): boolean {
	const view = session.view;
	if (!view) return false;
	if (!session.canEdit) {
		toast.info('This workbook is read-only for you');
		return true;
	}
	if (action.type === 'edit') edit.begin(view, 'cell');
	else if (action.type === 'type') edit.begin(view, 'cell', action.text);
	else if (action.type === 'clear') commands.clear();
	else if (action.type === 'undo' || action.type === 'redo') commands.history(action.type === 'redo');
	else if (action.type === 'toggle') commands.toggle(action.style);
	return true;
}

function moving(action: GridAction, { session, commands }: GridContext): boolean {
	const view = session.view;
	if (!view) return false;
	if (action.type === 'move' || action.type === 'expand' || action.type === 'edge') {
		commands.navigate(action.type, action.arrow);
	} else if (action.type === 'page') {
		commands.select((model) => (action.down ? model.onPageDown() : model.onPageUp()));
	} else if (action.type === 'selectAll') {
		commands.select((model) => model.setSelectedRange(1, 1, LAST_ROW, LAST_COLUMN));
	} else {
		return false;
	}
	return true;
}

/** Runs the action a key press on the grid maps to. Keys inside the cell editor never get here. */
export function gridKeydown(event: KeyboardEvent, context: GridContext): void {
	if (context.edit.target || event.isComposing) return;
	const action = gridAction(event);
	if (!action) return;
	if (moving(action, context) || editing(action, context)) event.preventDefault();
}
