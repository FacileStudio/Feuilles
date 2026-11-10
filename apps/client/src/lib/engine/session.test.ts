import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { emptySnapshot, load, loadEngineSync } from '../dev/engine-node';
import { Peer, pushSeqs } from '../dev/network';
import { FakeRoom } from '../dev/room';
import { FrameType, Role } from './protocol';
import { encodeInit, encodePresence, encodeReject, encodeUpdate } from './protocol-server';

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString('hex');
let room: FakeRoom;

function joined(role: (typeof Role)[keyof typeof Role] = Role.Editor, origin?: Uint8Array): Peer {
	const peer = new Peer(room, load, { role, origin });
	peer.join();
	peer.link.upstream = [];
	return peer;
}

const types = (frames: Uint8Array[]) => frames.map((frame) => frame[0]);

beforeAll(loadEngineSync);
beforeEach(() => {
	room = new FakeRoom(load, emptySnapshot());
});

describe('Session', () => {
	it('opens on INIT with the role and client id it carries', () => {
		const peer = joined();
		expect(peer.session.status).toBe('open');
		expect(peer.session.role).toBe(Role.Editor);
		expect(peer.session.clientId).toBe(peer.link.clientId);
	});

	it('never sends an empty batch', () => {
		const peer = joined();
		expect(peer.session.edit((model) => model.setUserInput(9, 1, 1, 'x'))).toBe(true);
		expect(types(peer.link.upstream)).not.toContain(FrameType.Push);
		expect(peer.session.replica!.pending).toEqual([]);
	});

	it('pushes an edit and clears it on its own echo', () => {
		const peer = joined();
		peer.session.edit((model) => model.setUserInput(0, 1, 1, '42'));
		expect(types(peer.link.upstream)).toEqual([FrameType.Push]);
		peer.link.deliver();
		peer.link.receive();
		expect(peer.session.replica!.pending).toEqual([]);
		expect(hex(peer.session.replica!.confirmed.toBytes())).toBe(hex(room.model.toBytes()));
	});

	it('rebuilds the view without a rejected batch and tells the user', () => {
		const peer = joined();
		peer.session.edit((model) => model.setUserInput(0, 1, 1, 'refused'));
		peer.link.downstream.push(encodeReject(1, 'rate_limited'));
		peer.link.receive();
		expect(peer.session.notice).toBe('rate_limited');
		expect(peer.session.view!.getCellContent(0, 1, 1)).toBe('');
		expect(peer.session.replica!.pending).toEqual([]);
	});

	it('drops a pending batch that no longer replays after a reject', () => {
		const peer = joined();
		peer.session.edit((model) => model.newSheet());
		peer.session.edit((model) => model.setUserInput(1, 1, 1, 'on the new sheet'));
		expect(peer.session.replica!.pending).toHaveLength(2);
		peer.link.downstream.push(encodeReject(1, 'refused'));
		peer.link.receive();
		expect(peer.session.replica!.pending).toEqual([]);
		expect(peer.session.view!.getWorksheetsProperties()).toHaveLength(1);
	});

	it('applies a remote batch, and an API write with client id 0', () => {
		const alice = joined();
		const bob = joined();
		alice.session.edit((model) => model.setUserInput(0, 1, 1, '2'));
		alice.link.deliver();
		room.write((model) => model.setUserInput(0, 2, 1, '=A1*3'));
		bob.link.receive();
		bob.link.receive();
		expect(bob.session.view!.getFormattedCellValue(0, 2, 1)).toBe('6');
	});

	it('reconnects after a drop and resends unacknowledged batches under their own client_seq', () => {
		const peer = joined();
		peer.session.edit((model) => model.setUserInput(0, 1, 1, 'a'));
		peer.session.edit((model) => model.setUserInput(0, 1, 2, 'b'));
		peer.link.drop();
		expect(peer.session.status).toBe('reconnecting');
		peer.session.edit((model) => model.setUserInput(0, 1, 3, 'c'));
		peer.clock.advance(1_000);
		peer.join();
		expect(pushSeqs(peer.link.upstream)).toEqual([1, 2, 3]);
		peer.settle();
		expect(room.model.getCellContent(0, 1, 3)).toBe('c');
		expect(peer.session.replica!.pending).toEqual([]);
	});

	it('resyncs when an UPDATE skips a sequence number', () => {
		const peer = joined();
		peer.link.downstream.push(encodeUpdate(5, 0, 0, new Uint8Array([0])));
		peer.link.receive();
		expect(peer.session.status).toBe('reconnecting');
		peer.clock.advance(1_000);
		peer.join();
		expect(peer.session.status).toBe('open');
	});

	it('stops for good when the server closes with 1008', () => {
		const peer = joined();
		peer.link.onclose?.({ code: 1008, reason: 'not_found' });
		peer.clock.advance(60_000);
		expect(peer.session.status).toBe('revoked');
		expect(peer.session.revokedReason).toBe('no_access');
		expect(peer.links).toHaveLength(1);
		expect(peer.session.canEdit).toBe(false);
	});

	it('refuses edits from a viewer', () => {
		const peer = joined(Role.Viewer);
		expect(peer.session.canEdit).toBe(false);
		expect(peer.session.edit((model) => model.setUserInput(0, 1, 1, 'x'))).toBe(false);
	});

	it('lists other people only in presence', () => {
		const peer = joined();
		const me = peer.session.clientId!;
		const cursor = { sheet: 0, row: 2, column: 3, range: [2, 3, 2, 3] as [number, number, number, number] };
		peer.link.downstream.push(
			encodePresence([
				{ client_id: me, name: 'Me', picture: '', cursor: null },
				{ client_id: 99, name: 'Zoé', picture: '', cursor }
			])
		);
		peer.link.receive();
		expect(peer.session.presence.map((entry) => entry.name)).toEqual(['Zoé']);
	});

	it('throttles cursor frames to ten a second and keeps the socket alive', () => {
		const peer = joined();
		peer.clock.advance(1_000);
		const cursor = (row: number) => ({ sheet: 0, row, column: 1, range: [row, 1, row, 1] as [number, number, number, number] });
		peer.session.setCursor(cursor(2));
		peer.session.setCursor(cursor(3));
		peer.session.setCursor(cursor(4));
		expect(peer.link.upstream).toHaveLength(1);
		peer.clock.advance(100);
		expect(peer.link.upstream).toHaveLength(2);
		expect(new TextDecoder().decode(peer.link.upstream[1].subarray(1))).toContain('"row":4');
		peer.link.upstream = [];
		peer.clock.advance(25_000);
		expect(types(peer.link.upstream)).toEqual([FrameType.Cursor]);
	});

	it('replaces confirmed on a second INIT from the same socket', () => {
		const peer = joined();
		const other = new FakeRoom(load, emptySnapshot());
		other.write((model) => model.setUserInput(0, 1, 1, 'fresh'));
		peer.link.downstream.push(encodeInit({ seq: other.seq, clientId: 4, role: Role.Editor, acked: 0 }, other.model.toBytes()));
		peer.link.receive();
		expect(peer.session.view!.getCellContent(0, 1, 1)).toBe('fresh');
		expect(peer.session.replica!.seq).toBe(1);
	});
});
