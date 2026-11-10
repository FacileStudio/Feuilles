import type { Area, HorizontalAlignment, Model } from '@ironcalc/wasm';
import type { Session } from '$lib/engine/session.svelte';
import type { Arrow, Toggle } from './keyboard';

/** The number formats of the v1 toolbar (plan 7.7), as IronCalc format strings. */
export const NUMBER_FORMATS = {
	general: 'general',
	number: '#,##0.00',
	percent: '0.00%',
	currency: '"€"#,##0.00',
	date: 'yyyy-mm-dd'
} as const;

/** A toolbar number format. */
export type NumberFormat = keyof typeof NUMBER_FORMATS;

const styleKeys: Record<Toggle, 'b' | 'i' | 'u'> = { bold: 'b', italic: 'i', underline: 'u' };

/** The selected range of `model`, as the `Area` IronCalc's range calls take. */
export function selectedArea(model: Model): Area {
	const { sheet, range } = model.getSelectedView();
	const [r1, c1, r2, c2] = range;
	return { sheet, row: Math.min(r1, r2), column: Math.min(c1, c2), width: Math.abs(c2 - c1) + 1, height: Math.abs(r2 - r1) + 1 };
}

/**
 * Every change the grid chrome can make. Each one goes through `Session.edit`, so it lands in one
 * batch, is pushed, and is undone or refused like any other edit. Navigation only moves the
 * selection, which IronCalc keeps out of the send queue, so it calls `touch` to redraw.
 */
export class GridCommands {
	constructor(private readonly session: Session) {}

	/** Sets one style path on the selection. */
	style(path: string, value: string): boolean {
		return this.session.edit((model) => model.updateRangeStyle(selectedArea(model), path, value));
	}

	/** Flips bold, italic or underline, reading the state of the active cell. */
	toggle(which: Toggle): boolean {
		const view = this.session.view;
		if (!view) return false;
		const [sheet, row, column] = view.getSelectedCell();
		const on = view.getCellStyle(sheet, row, column).style.font[styleKeys[which]];
		return this.style(`font.${styleKeys[which]}`, String(!on));
	}

	/** Aligns the selection. */
	align(horizontal: HorizontalAlignment): boolean {
		return this.style('alignment.horizontal', horizontal);
	}

	/** Applies a toolbar number format. */
	format(kind: NumberFormat): boolean {
		return this.style('num_fmt', NUMBER_FORMATS[kind]);
	}

	/** Writes one cell, as a commit from the cell editor or the formula bar. */
	write(sheet: number, row: number, column: number, input: string): boolean {
		return this.session.edit((model) => model.setUserInput(sheet, row, column, input));
	}

	/** Empties the selection, keeping its formatting. */
	clear(): boolean {
		return this.session.edit((model) => {
			const { sheet, row, column, width, height } = selectedArea(model);
			model.rangeClearContents(sheet, row, column, row + height - 1, column + width - 1);
		});
	}

	/** Undoes or redoes the last local edit. */
	history(redo: boolean): boolean {
		return this.session.edit((model) => (redo ? model.redo() : model.undo()));
	}

	/** Inserts as many rows as are selected, above or below the selection. */
	insertRows(below: boolean): boolean {
		return this.session.edit((model) => {
			const { sheet, row, height } = selectedArea(model);
			model.insertRows(sheet, below ? row + height : row, height);
		});
	}

	/** Inserts as many columns as are selected, left or right of the selection. */
	insertColumns(right: boolean): boolean {
		return this.session.edit((model) => {
			const { sheet, column, width } = selectedArea(model);
			model.insertColumns(sheet, right ? column + width : column, width);
		});
	}

	/** Deletes the selected rows or columns. */
	deleteLines(columns: boolean): boolean {
		return this.session.edit((model) => {
			const { sheet, row, column, width, height } = selectedArea(model);
			if (columns) model.deleteColumns(sheet, column, width);
			else model.deleteRows(sheet, row, height);
		});
	}

	/** Freezes the rows above and the columns left of the active cell, or unfreezes when panes are frozen. */
	freeze(): boolean {
		return this.session.edit((model) => {
			const [sheet, row, column] = model.getSelectedCell();
			const frozen = model.getFrozenRowsCount(sheet) + model.getFrozenColumnsCount(sheet) > 0;
			model.setFrozenRowsCount(sheet, frozen ? 0 : row - 1);
			model.setFrozenColumnsCount(sheet, frozen ? 0 : column - 1);
		});
	}

	/** Resizes columns or rows, from the canvas's drag handles. */
	resize(sheet: number, start: number, end: number, size: number, columns: boolean): boolean {
		if (size < 0) return false;
		return this.session.edit((model) =>
			columns ? model.setColumnsWidth(sheet, start, end, size) : model.setRowsHeight(sheet, start, end, size)
		);
	}

	/** Pushes whatever the canvas changed on the view by itself, such as an autofill drag. */
	flush(): boolean {
		return this.session.edit(() => undefined);
	}

	/** Adds a sheet at the end and shows it. */
	addSheet(): boolean {
		return this.session.edit((model) => {
			model.newSheet();
			model.setSelectedSheet(model.getWorksheetsProperties().length - 1);
		});
	}

	/** Renames, deletes or moves a sheet. */
	sheet(index: number, change: { rename: string } | { remove: true } | { moveTo: number }): boolean {
		return this.session.edit((model) => {
			if ('rename' in change) model.renameSheet(index, change.rename);
			else if ('remove' in change) model.deleteSheet(index);
			else model.moveSheet(index, change.moveTo);
		});
	}

	/** Moves, extends or jumps the selection. */
	navigate(kind: 'move' | 'expand' | 'edge', arrow: Arrow): void {
		const view = this.session.view;
		if (!view) return;
		if (kind === 'expand') view.onExpandSelectedRange(arrow);
		else if (kind === 'edge') view.onNavigateToEdgeInDirection(arrow);
		else view[`on${arrow}` as 'onArrowUp']();
		this.session.touch();
	}

	/** Runs a selection-only change on the view and redraws. */
	select(change: (model: Model) => void): void {
		const view = this.session.view;
		if (!view) return;
		change(view);
		this.session.touch();
	}
}
