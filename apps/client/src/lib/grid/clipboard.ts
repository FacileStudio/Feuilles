import type { ClipboardCell, Model } from '@ironcalc/wasm';

/** The MIME type an internal copy travels under, beside `text/plain`. */
export const INTERNAL_MIME = 'application/x-feuilles-cells+json';

type Grid = Record<number, Record<number, ClipboardCell>>;

/** An internal copy: the source sheet and range, its cells, and whether it was a cut. */
export type Copied = { type: 'copy' | 'cut'; sheet: number; area: [number, number, number, number]; cells: Grid };

/** What a copy writes to the system clipboard. */
export type ClipboardPayload = { text: string; json: string };

/** Copies the selection of `model`: tab-separated text for other apps, the full cells for Feuilles. */
export function copySelection(model: Model, type: 'copy' | 'cut'): ClipboardPayload {
	const clip = model.copyToClipboard();
	const cells: Grid = {};
	for (const [row, columns] of clip.data) {
		cells[row] = Object.fromEntries(columns);
	}
	const copied: Copied = { type, sheet: model.getSelectedSheet(), area: clip.range, cells };
	return { text: clip.csv.trim(), json: JSON.stringify(copied) };
}

function toMap(cells: Grid): Map<number, Map<number, ClipboardCell>> {
	const rows = Object.entries(cells).map(([row, columns]) => {
		const entries = Object.entries(columns).map(([column, cell]) => [Number(column), cell] as const);
		return [Number(row), new Map(entries)] as const;
	});
	return new Map(rows);
}

/** Pastes at the selection: Feuilles cells when the clipboard has them, plain text otherwise. */
export function pasteInto(model: Model, json: string | null, text: string): void {
	if (json) {
		const copied = JSON.parse(json) as Copied;
		return model.pasteFromClipboard(copied.sheet, copied.area, toMap(copied.cells), copied.type === 'cut');
	}
	const { sheet, range } = model.getSelectedView();
	const [r1, c1, r2, c2] = range;
	const area = { sheet, row: Math.min(r1, r2), column: Math.min(c1, c2), width: Math.abs(c2 - c1) + 1, height: Math.abs(r2 - r1) + 1 };
	model.pasteCsvText(area, text);
}
