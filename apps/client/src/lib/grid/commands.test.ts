import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { emptySnapshot, load, loadEngineSync } from '../dev/engine-node';
import { Peer } from '../dev/network';
import { FakeRoom } from '../dev/room';
import { FrameType } from '../engine/protocol';
import { GridCommands } from './commands';

let room: FakeRoom;
let peer: Peer;
let commands: GridCommands;

function settle() {
	while (peer.link.upstream.length || peer.link.downstream.length) {
		peer.link.deliver();
		peer.link.receive();
	}
}

beforeAll(loadEngineSync);
beforeEach(() => {
	room = new FakeRoom(load, emptySnapshot());
	peer = new Peer(room, load);
	peer.join();
	commands = new GridCommands(peer.session);
});

describe('GridCommands', () => {
	it('toggles bold on the selection through one pushed batch', () => {
		peer.session.view!.setSelectedRange(1, 1, 2, 2);
		peer.link.upstream = [];
		commands.toggle('bold');
		expect(peer.link.upstream.map((frame) => frame[0])).toEqual([FrameType.Push]);
		settle();
		expect(room.model.getCellStyle(0, 2, 2).style.font.b).toBe(true);
		commands.toggle('bold');
		settle();
		expect(room.model.getCellStyle(0, 1, 1).style.font.b).toBeFalsy();
	});

	it('applies number formats and alignment', () => {
		commands.write(0, 1, 1, '0.25');
		commands.format('percent');
		commands.align('center');
		settle();
		expect(room.model.getFormattedCellValue(0, 1, 1)).toBe('25.00%');
		expect(room.model.getCellStyle(0, 1, 1).style.alignment?.horizontal).toBe('center');
	});

	it('inserts and deletes as many rows as are selected', () => {
		commands.write(0, 3, 1, 'kept');
		peer.session.view!.setSelectedRange(1, 1, 2, 1);
		commands.insertRows(false);
		settle();
		expect(room.model.getCellContent(0, 5, 1)).toBe('kept');
		commands.deleteLines(false);
		settle();
		expect(room.model.getCellContent(0, 3, 1)).toBe('kept');
	});

	it('freezes at the active cell, then unfreezes', () => {
		peer.session.view!.setSelectedCell(3, 2);
		commands.freeze();
		settle();
		expect([room.model.getFrozenRowsCount(0), room.model.getFrozenColumnsCount(0)]).toEqual([2, 1]);
		commands.freeze();
		settle();
		expect(room.model.getFrozenRowsCount(0)).toBe(0);
	});

	it('adds, renames, moves and deletes sheets', () => {
		commands.addSheet();
		commands.sheet(1, { rename: 'Data' });
		commands.sheet(1, { moveTo: 0 });
		settle();
		expect(room.model.getWorksheetsProperties().map((sheet) => sheet.name)).toEqual(['Data', 'Sheet1']);
		commands.sheet(0, { remove: true });
		settle();
		expect(room.model.getWorksheetsProperties()).toHaveLength(1);
	});

	it('undoes and redoes a local edit', () => {
		commands.write(0, 1, 1, 'x');
		commands.history(false);
		settle();
		expect(room.model.getCellContent(0, 1, 1)).toBe('');
		commands.history(true);
		settle();
		expect(room.model.getCellContent(0, 1, 1)).toBe('x');
	});

	it('navigates without pushing anything', () => {
		peer.link.upstream = [];
		commands.navigate('move', 'ArrowDown');
		commands.navigate('expand', 'ArrowRight');
		expect(peer.session.view!.getSelectedView().range).toEqual([2, 1, 2, 2]);
		expect(peer.link.upstream.filter((frame) => frame[0] === FrameType.Push)).toEqual([]);
	});
});
