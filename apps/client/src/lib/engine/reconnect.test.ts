import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { emptySnapshot, load, loadEngineSync } from '../dev/engine-node';
import { Peer, pushSeqs } from '../dev/network';
import { FakeRoom } from '../dev/room';
import { FrameType, Role } from './protocol';

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString('hex');
let room: FakeRoom;

function joined(role: (typeof Role)[keyof typeof Role] = Role.Editor, origin?: Uint8Array): Peer {
	const peer = new Peer(room, load, { role, origin });
	peer.join();
	peer.link.upstream = [];
	return peer;
}

beforeAll(loadEngineSync);
beforeEach(() => {
	room = new FakeRoom(load, emptySnapshot());
});

describe('Session reconnects (docs/api.md, session flow and close codes)', () => {
	it('sends HELLO with its origin first, and the same origin after a reconnect', () => {
		const origin = Uint8Array.from({ length: 16 }, (_, i) => 0xa0 + i);
		const peer = new Peer(room, load, { origin });
		expect(Array.from(peer.link.upstream[0])).toEqual([FrameType.Hello, ...origin]);
		peer.join();
		peer.link.drop();
		peer.clock.advance(1_000);
		expect(peer.links).toHaveLength(2);
		expect(Array.from(peer.link.upstream[0])).toEqual([FrameType.Hello, ...origin]);
	});

	it('never resends a batch the server accepted when its echo was lost', () => {
		const peer = joined();
		peer.session.edit((model) => model.setUserInput(0, 1, 1, 'kept'));
		peer.session.edit((model) => model.insertRows(0, 1, 1));
		peer.session.edit((model) => model.setUserInput(0, 5, 1, 'late'));
		peer.link.deliver();
		peer.link.deliver();
		expect(room.seq).toBe(2);
		peer.link.drop();
		peer.clock.advance(1_000);
		peer.join();
		expect(pushSeqs(peer.link.upstream)).toEqual([3]);
		peer.settle();
		expect(room.resent).toBe(0);
		expect(room.seq).toBe(3);
		expect(room.model.getCellContent(0, 2, 1)).toBe('kept');
		expect(room.model.getCellContent(0, 3, 1)).toBe('');
		expect(hex(peer.session.view!.toBytes())).toBe(hex(room.model.toBytes()));
	});

	it('numbers new batches above what the server acked for its origin, after a reload', () => {
		const origin = new Uint8Array(16).fill(7);
		const first = joined(Role.Editor, origin);
		first.session.edit((model) => model.setUserInput(0, 1, 1, 'one'));
		first.session.edit((model) => model.setUserInput(0, 1, 2, 'two'));
		first.settle();
		first.session.close();
		const reloaded = joined(Role.Editor, origin);
		reloaded.session.edit((model) => model.setUserInput(0, 1, 3, 'three'));
		expect(pushSeqs(reloaded.link.upstream)).toEqual([3]);
		reloaded.settle();
		expect(room.model.getCellContent(0, 1, 3)).toBe('three');
		expect(reloaded.session.replica!.pending).toEqual([]);
	});

	it('reconnects at once on 1012 and with backoff on 1013 and 1001', () => {
		const peer = joined();
		peer.link.drop(1012);
		peer.clock.advance(0);
		expect(peer.links).toHaveLength(2);
		for (const code of [1013, 1001]) {
			peer.join();
			const before = peer.links.length;
			peer.link.drop(code);
			peer.clock.advance(100);
			expect(peer.links, `code ${code}`).toHaveLength(before);
			peer.clock.advance(10_000);
			expect(peer.links, `code ${code}`).toHaveLength(before + 1);
		}
	});

	it('drops the batch that closed the socket with 1009 and tells the user', () => {
		const peer = joined();
		peer.session.edit((model) => model.setUserInput(0, 1, 1, 'accepted'));
		peer.link.deliver();
		peer.session.edit((model) => model.setUserInput(0, 1, 2, 'too big'));
		peer.session.edit((model) => model.setUserInput(0, 1, 3, 'after'));
		peer.link.drop(1009);
		peer.clock.advance(1_000);
		peer.join();
		expect(pushSeqs(peer.link.upstream)).toEqual([3]);
		expect(peer.session.notice).toBe('too_large');
		peer.settle();
		expect(room.model.getCellContent(0, 1, 1)).toBe('accepted');
		expect(room.model.getCellContent(0, 1, 2)).toBe('');
		expect(room.model.getCellContent(0, 1, 3)).toBe('after');
	});

	it('asks to sign in again when 1008 says unauthenticated', () => {
		const peer = joined();
		peer.link.onclose?.({ code: 1008, reason: 'unauthenticated' });
		expect(peer.session.revokedReason).toBe('signed_out');
	});
});
