import { beforeAll, describe, expect, it } from 'vitest';
import { emptySnapshot, load, loadEngineSync } from '../dev/engine-node';
import { copySelection, pasteInto } from './clipboard';

beforeAll(loadEngineSync);

function filled() {
	const model = load(emptySnapshot());
	model.setUserInput(0, 1, 1, '1');
	model.setUserInput(0, 1, 2, '2');
	model.setUserInput(0, 2, 1, '=A1+B1');
	model.updateRangeStyle({ sheet: 0, row: 1, column: 1, width: 1, height: 1 }, 'font.b', 'true');
	model.setSelectedCell(1, 1);
	model.setSelectedRange(1, 1, 2, 2);
	return model;
}

describe('clipboard', () => {
	it('copies tab-separated text for other apps', () => {
		expect(copySelection(filled(), 'copy').text).toBe('1\t2\n3');
	});

	it('pastes Feuilles cells with formulas shifted and styles kept', () => {
		const source = filled();
		const { json } = copySelection(source, 'copy');
		const target = load(emptySnapshot());
		target.setSelectedCell(5, 3);
		pasteInto(target, json, '');
		expect(target.getCellContent(0, 6, 3)).toBe('=C5+D5');
		expect(target.getFormattedCellValue(0, 6, 3)).toBe('3');
		expect(target.getCellStyle(0, 5, 3).style.font.b).toBe(true);
	});

	it('pastes plain text from another app at the selection', () => {
		const target = load(emptySnapshot());
		target.setSelectedCell(2, 2);
		pasteInto(target, null, 'a\tb\n4\t5');
		expect(target.getCellContent(0, 2, 3)).toBe('b');
		expect(target.getFormattedCellValue(0, 3, 2)).toBe('4');
	});
});
