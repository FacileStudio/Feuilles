import type { Model } from '@ironcalc/wasm';
import { getEditorSize } from './canvas/vendor/components/util';
import { WorkbookState } from './canvas/vendor/components/workbookState';
import type { GridCommands } from './commands';
import type { Arrow } from './keyboard';

/** Where an edit is typed: in the cell itself or in the formula bar. */
export type EditFocus = 'cell' | 'formula-bar';

/** The cell being edited. */
export type EditTarget = { sheet: number; row: number; column: number; focus: EditFocus };

/**
 * The in-progress cell edit, shared by the cell editor, the formula bar and the canvas. The text
 * lives here until a commit writes it through `GridCommands.write`, so a keystroke never reaches
 * the server and only the committed value does.
 */
export class CellEdit {
	readonly state = new WorkbookState();
	target = $state<EditTarget | null>(null);
	text = $state('');

	/** Starts editing the active cell. `replace` starts from that text instead of the cell's content. */
	begin(view: Model, focus: EditFocus, replace?: string): void {
		const [sheet, row, column] = view.getSelectedCell();
		const text = replace ?? view.getCellContent(sheet, row, column);
		const { width, height } = getEditorSize(view, sheet, row, column);
		this.target = { sheet, row, column, focus };
		this.text = text;
		this.state.setEditingCell({
			sheet,
			row,
			column,
			text,
			cursorStart: text.length,
			cursorEnd: text.length,
			focus,
			referencedRange: null,
			activeRanges: [],
			mode: replace === undefined ? 'edit' : 'accept',
			editorWidth: width,
			editorHeight: height
		});
	}

	/** Records what the person typed. */
	set(text: string): void {
		this.text = text;
		const cell = this.state.getEditingCell();
		if (cell) Object.assign(cell, { text, cursorStart: text.length, cursorEnd: text.length });
	}

	/** Writes the text to the cell, ends the edit, and moves the selection when asked. */
	commit(commands: GridCommands, then: Arrow | null = null): void {
		const target = this.target;
		if (!target) return;
		const text = this.text;
		this.cancel();
		commands.write(target.sheet, target.row, target.column, text);
		if (then) commands.navigate('move', then);
	}

	/** Drops the edit. */
	cancel(): void {
		this.target = null;
		this.text = '';
		this.state.clearEditingCell();
	}
}
