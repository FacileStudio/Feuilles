/** An arrow key, named the way IronCalc's navigation calls name it. */
export type Arrow = 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight';

/** Formatting a shortcut toggles. */
export type Toggle = 'bold' | 'italic' | 'underline';

/** What a key press on the grid (not inside the cell editor) asks for. */
export type GridAction =
	| { type: 'move'; arrow: Arrow }
	| { type: 'expand'; arrow: Arrow }
	| { type: 'edge'; arrow: Arrow }
	| { type: 'page'; down: boolean }
	| { type: 'edit' }
	| { type: 'type'; text: string }
	| { type: 'clear' }
	| { type: 'undo' }
	| { type: 'redo' }
	| { type: 'toggle'; style: Toggle }
	| { type: 'selectAll' };

/** What a key press inside the cell editor asks for. */
export type EditorAction =
	| { type: 'commit'; then: Arrow | null }
	| { type: 'cancel' }
	| { type: 'newline' }
	| { type: 'none' };

/** The fields of a `KeyboardEvent` the mapping reads. */
export type KeyPress = { key: string; shiftKey: boolean; ctrlKey: boolean; metaKey: boolean; altKey: boolean };

const arrows = new Set<string>(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight']);
const toggles: Record<string, Toggle> = { b: 'bold', i: 'italic', u: 'underline' };

function modified(press: KeyPress, key: string): GridAction | null {
	if (key === 'z') return { type: press.shiftKey ? 'redo' : 'undo' };
	if (key === 'y') return { type: 'redo' };
	if (key === 'a') return { type: 'selectAll' };
	if (toggles[key]) return { type: 'toggle', style: toggles[key] };
	return arrows.has(press.key) ? { type: 'edge', arrow: press.key as Arrow } : null;
}

function plain(press: KeyPress): GridAction | null {
	if (arrows.has(press.key)) return { type: press.shiftKey ? 'expand' : 'move', arrow: press.key as Arrow };
	if (press.key === 'Tab') return { type: 'move', arrow: press.shiftKey ? 'ArrowLeft' : 'ArrowRight' };
	if (press.key === 'Enter' || press.key === 'F2') return { type: 'edit' };
	if (press.key === 'Delete' || press.key === 'Backspace') return { type: 'clear' };
	if (press.key === 'PageDown' || press.key === 'PageUp') return { type: 'page', down: press.key === 'PageDown' };
	return [...press.key].length === 1 && !press.altKey ? { type: 'type', text: press.key } : null;
}

/** Maps a key press on the grid to an action, or null when the grid leaves the key alone. */
export function gridAction(press: KeyPress): GridAction | null {
	const mod = press.ctrlKey || press.metaKey;
	return mod ? modified(press, press.key.toLowerCase()) : plain(press);
}

/**
 * Maps a key press inside the cell editor. Enter commits and moves down, Tab right, and their
 * Shift forms the other way; Alt+Enter inserts a line break; Escape drops the edit.
 */
export function editorAction(press: KeyPress): EditorAction {
	if (press.key === 'Escape') return { type: 'cancel' };
	if (press.key === 'Enter' && press.altKey) return { type: 'newline' };
	if (press.key === 'Enter') return { type: 'commit', then: press.shiftKey ? 'ArrowUp' : 'ArrowDown' };
	if (press.key === 'Tab') return { type: 'commit', then: press.shiftKey ? 'ArrowLeft' : 'ArrowRight' };
	return { type: 'none' };
}
