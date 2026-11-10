import type { Model } from '@ironcalc/wasm';
import type { Cell } from './canvas/vendor/components/types';
import { headerColumnWidth, headerRowHeight, LAST_COLUMN, LAST_ROW } from './canvas/vendor/components/WorksheetCanvas/constants';
import type WorksheetCanvas from './canvas/vendor/components/WorksheetCanvas/worksheetCanvas';

/** Which part of the sheet a pointer is over. */
export type Zone = 'corner' | 'rows' | 'columns' | 'cells' | 'outside';

/** Classifies a point in canvas coordinates. */
export function zoneAt(x: number, y: number, width: number, height: number): Zone {
	if (x < 0 || y < 0 || x > width || y > height) return 'outside';
	if (x < headerColumnWidth && y < headerRowHeight) return 'corner';
	if (x < headerColumnWidth) return 'rows';
	return y < headerRowHeight ? 'columns' : 'cells';
}

type Drag = { kind: 'cells' } | { kind: 'rows'; from: number } | { kind: 'columns'; from: number };

/** What the pointer controller needs from the grid. */
export type PointerHost = {
	canvas: () => WorksheetCanvas | null;
	view: () => Model | null;
	element: () => HTMLElement | null;
	beforeSelect: () => void;
	changed: () => void;
};

/**
 * Selection with the mouse, ported from upstream's React `usePointer`: a click selects a cell,
 * dragging selects a range, the headers select whole rows and columns, Shift extends. Picking a
 * reference with the mouse while typing a formula is not ported in v1.
 */
export class GridPointer {
	private drag: Drag | null = null;

	constructor(private readonly host: PointerHost) {}

	/** Handles `pointerdown` on the sheet container. */
	down(event: PointerEvent): void {
		const target = event.target as HTMLElement;
		if (event.button === 2 || /column-resize-handle|row-resize-handle|ironcalc-cell-handle/.test(target.className)) return;
		const hit = this.locate(event);
		if (!hit) return;
		this.host.beforeSelect();
		if (hit.zone === 'outside') return;
		if (hit.zone === 'corner') return this.selectAll();
		if (hit.zone === 'cells') return this.downOnCell(hit.cell, event);
		const line = hit.zone === 'rows' ? hit.cell.row : hit.cell.column;
		this.selectLines(hit.zone, event.shiftKey ? null : line, line);
		if (!event.shiftKey) this.start(event, { kind: hit.zone, from: line });
	}

	/** Handles `pointermove`: extends the selection while a drag is running. */
	move(event: PointerEvent): void {
		const drag = this.drag;
		const hit = drag ? this.locate(event) : null;
		if (!drag || !hit || hit.zone === 'outside' || hit.zone === 'corner') return;
		if (drag.kind === 'cells') this.view()?.onAreaSelecting(hit.cell.row, hit.cell.column);
		else this.selectLines(drag.kind, drag.from, drag.kind === 'rows' ? hit.cell.row : hit.cell.column);
		this.host.changed();
	}

	/** Handles `pointerup`: ends the drag. */
	up(event: PointerEvent): void {
		if (!this.drag) return;
		this.drag = null;
		this.host.element()?.releasePointerCapture?.(event.pointerId);
		this.host.changed();
	}

	/** The cell under a client point, for the context menu and double click. */
	cellAt(clientX: number, clientY: number): { zone: Zone; cell: Cell } | null {
		const canvas = this.host.canvas();
		if (!canvas) return null;
		const rect = canvas.canvas.getBoundingClientRect();
		const x = clientX - rect.x;
		const y = clientY - rect.y;
		const zone = zoneAt(x, y, rect.width, rect.height);
		const cell = canvas.getCellByCoordinates(Math.max(x, headerColumnWidth), Math.max(y, headerRowHeight));
		return cell ? { zone, cell } : null;
	}

	private locate(event: PointerEvent) {
		return this.cellAt(event.clientX, event.clientY);
	}

	private view(): Model | null {
		return this.host.view();
	}

	private start(event: PointerEvent, drag: Drag): void {
		this.drag = drag;
		this.host.element()?.setPointerCapture?.(event.pointerId);
	}

	private downOnCell(cell: Cell, event: PointerEvent): void {
		const view = this.view();
		if (!view) return;
		if (event.shiftKey) view.onAreaSelecting(cell.row, cell.column);
		else view.setSelectedCell(cell.row, cell.column);
		if (!event.shiftKey) this.start(event, { kind: 'cells' });
		this.host.changed();
	}

	private selectAll(): void {
		const view = this.view();
		view?.setSelectedCell(1, 1);
		view?.setSelectedRange(1, 1, LAST_ROW, LAST_COLUMN);
		this.host.changed();
	}

	private selectLines(kind: 'rows' | 'columns', from: number | null, to: number): void {
		const view = this.view();
		if (!view) return;
		const { range } = view.getSelectedView();
		const anchor = from ?? (kind === 'rows' ? range[0] : range[1]);
		const [low, high] = [Math.min(anchor, to), Math.max(anchor, to)];
		if (kind === 'rows') {
			view.setSelectedCell(low, 1);
			view.setSelectedRange(low, 1, high, LAST_COLUMN);
		} else {
			view.setSelectedCell(1, low);
			view.setSelectedRange(1, low, LAST_ROW, high);
		}
		this.host.changed();
	}
}
