import type { Model } from '@ironcalc/wasm';
import { beforeAll, describe, expect, it } from 'vitest';
import { emptySnapshot, load, loadEngineSync } from '../dev/engine-node';
import { Peer } from '../dev/network';
import { FakeRoom } from '../dev/room';

const CASES = Number(process.env.SESSION_CASES ?? 1000);
const CLIENTS = 3;
const MAX_STEPS = 40;

function rng(seed: number): () => number {
	let state = seed >>> 0;
	return () => {
		state = (state + 0x6d2b79f5) >>> 0;
		let t = state;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

const inputs = ['1', '2.5', '-7', 'hello', '', '=A1+B2', '=SUM(A1:C3)', '=Sheet2!A1', '=B!A1&"x"', '=1/0', '=(', '=A1*2'];
const styles: [string, string][] = [
	['font.b', 'true'],
	['font.i', 'true'],
	['fill.fg_color', '#FFE699'],
	['num_fmt', '0.00%'],
	['alignment.horizontal', 'center']
];

type Pick = <T>(items: readonly T[]) => T;

function randomEdit(pick: Pick, int: (n: number) => number): (model: Model) => void {
	const kind = int(12);
	return (model) => {
		const sheet = int(model.getWorksheetsProperties().length + 1);
		const row = 1 + int(6);
		const column = 1 + int(6);
		if (kind < 5) return model.setUserInput(sheet, row, column, pick(inputs));
		if (kind === 5) return model.updateRangeStyle({ sheet, row, column, width: 1 + int(3), height: 1 + int(3) }, ...pick(styles));
		if (kind === 6) return int(2) ? model.insertRows(sheet, row, 1) : model.deleteRows(sheet, row, 1);
		if (kind === 7) return int(2) ? model.insertColumns(sheet, column, 1) : model.deleteColumns(sheet, column, 1);
		if (kind === 8) return model.newSheet();
		if (kind === 9) return model.renameSheet(sheet, pick(['A', 'B', 'Sheet2', 'Données']));
		if (kind === 10) return model.undo();
		return model.setColumnsWidth(sheet, column, column, 40 + int(100));
	};
}

function runCase(seed: number) {
	const random = rng(seed);
	const int = (n: number) => Math.floor(random() * n);
	const pick: Pick = (items) => items[int(items.length)];
	const room = new FakeRoom(load, emptySnapshot());
	const peers = Array.from({ length: CLIENTS }, () => new Peer(room, load, random));
	const steps = 1 + int(MAX_STEPS);
	for (let i = 0; i < steps; i += 1) {
		const peer = pick(peers);
		const roll = int(20);
		if (roll < 7) peer.session.edit(randomEdit(pick, int));
		else if (roll < 12) peer.link.deliver();
		else if (roll < 18) peer.link.receive();
		else if (roll === 18) peer.link.drop();
		else peer.clock.advance(20_000);
	}
	return { room, peers };
}

function drain(peers: Peer[]): void {
	for (let round = 0; round < 1000; round += 1) {
		const offline = peers.filter((peer) => peer.session.status !== 'open');
		for (const peer of offline) peer.clock.advance(11_000);
		if (offline.length === 0 && !peers.some((peer) => peer.busy)) return;
		for (const peer of peers) {
			peer.link.deliver();
			peer.link.receive();
		}
	}
	throw new Error('the room never settled');
}

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString('hex');

describe('Session convergence under random interleavings (plan 7.1)', () => {
	beforeAll(loadEngineSync);

	it(`${CASES} cases: every confirmed model equals the server, idle views equal confirmed`, () => {
		const totals = { rejected: 0, reconnects: 0, accepted: 0 };
		for (let seed = 1; seed <= CASES; seed += 1) {
			const { room, peers } = runCase(seed);
			drain(peers);
			totals.rejected += room.rejected;
			totals.accepted += room.seq;
			totals.reconnects += peers.reduce((sum, peer) => sum + peer.links.length - 1, 0);
			const server = hex(room.model.toBytes());
			for (const [index, peer] of peers.entries()) {
				const replica = peer.session.replica!;
				expect(replica.pending, `seed ${seed} peer ${index} pending`).toEqual([]);
				expect(replica.seq, `seed ${seed} peer ${index} seq`).toBe(room.seq);
				expect(hex(replica.confirmed.toBytes()), `seed ${seed} peer ${index} confirmed`).toBe(server);
				expect(hex(replica.view.toBytes()), `seed ${seed} peer ${index} view`).toBe(server);
			}
			for (const peer of peers) peer.session.close();
			room.model.free();
		}
		expect(totals.accepted).toBeGreaterThan(0);
		expect(totals.rejected).toBeGreaterThan(0);
		expect(totals.reconnects).toBeGreaterThan(0);
	}, 600_000);
});
