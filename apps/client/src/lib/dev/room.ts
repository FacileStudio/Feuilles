import type { Model } from '@ironcalc/wasm';
import { Role, type RoleValue } from '../engine/protocol';
import { decodeClientFrame, encodeInit, encodeReject, encodeUpdate } from '../engine/protocol-server';

type Member = { send: (frame: Uint8Array) => void; role: RoleValue };

/**
 * An in-memory server room following plan 7.1 and 7.3, for tests only: applies batches in
 * arrival order, broadcasts every accepted one to everyone including the sender, and on a refused
 * batch reloads its model from the last good bytes, since IronCalc's apply is not atomic.
 */
export class FakeRoom {
	model: Model;
	seq = 0;
	rejected = 0;
	private members = new Map<number, Member>();
	private nextId = 1;

	constructor(
		private readonly load: (bytes: Uint8Array) => Model,
		snapshot: Uint8Array
	) {
		this.model = load(snapshot);
	}

	/** Adds a connection and sends it INIT. Returns its client id. */
	join(send: (frame: Uint8Array) => void, role: RoleValue = Role.Editor): number {
		const id = this.nextId++;
		this.members.set(id, { send, role });
		send(encodeInit(this.seq, id, role, this.model.toBytes()));
		return id;
	}

	/** Removes a connection. */
	leave(clientId: number): void {
		this.members.delete(clientId);
	}

	/** Handles one client frame from `clientId`. */
	receive(clientId: number, bytes: Uint8Array): void {
		const member = this.members.get(clientId);
		const frame = decodeClientFrame(bytes);
		if (!member || frame.type !== 'push') return;
		if (member.role !== Role.Editor) return member.send(encodeReject(frame.clientSeq, 'permission_denied'));
		const before = this.model.toBytes();
		try {
			this.model.applyExternalDiffs(frame.payload);
		} catch (error) {
			this.rejected += 1;
			this.model.free();
			this.model = this.load(before);
			return member.send(encodeReject(frame.clientSeq, String(error)));
		}
		this.seq += 1;
		const update = encodeUpdate(this.seq, clientId, frame.clientSeq, frame.payload);
		for (const other of this.members.values()) other.send(update);
	}

	/** Applies a write made by the server itself (API or MCP), broadcast with client id 0. */
	write(change: (model: Model) => void): void {
		change(this.model);
		const payload = this.model.flushSendQueue();
		this.seq += 1;
		const update = encodeUpdate(this.seq, 0, 0, payload);
		for (const member of this.members.values()) member.send(update);
	}
}
