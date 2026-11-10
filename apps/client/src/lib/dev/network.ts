import type { Model } from '@ironcalc/wasm';
import { fresh } from '../engine/origin';
import { FrameType, Role, type RoleValue } from '../engine/protocol';
import { Session } from '../engine/session.svelte';
import type { Clock, SessionSocket } from '../engine/socket';
import type { FakeRoom } from './room';

/** A clock whose timers run only when a test says so. */
export class ManualClock implements Clock {
	time = 0;
	private timers = new Map<number, { run: () => void; at: number; every: number }>();
	private nextId = 1;

	constructor(private readonly seed: () => number = () => 0.5) {}

	now = () => this.time;
	random = () => this.seed();
	setTimeout = (run: () => void, ms: number) => this.add(run, ms, 0);
	setInterval = (run: () => void, ms: number) => this.add(run, ms, ms);
	clearTimeout = (id: unknown) => void this.timers.delete(id as number);
	clearInterval = (id: unknown) => void this.timers.delete(id as number);

	/** Moves time forward, running every timer that falls due, in order. */
	advance(ms: number): void {
		const end = this.time + ms;
		for (let next = this.due(end); next; next = this.due(end)) {
			const [id, timer] = next;
			this.time = timer.at;
			if (timer.every > 0) timer.at += timer.every;
			else this.timers.delete(id);
			timer.run();
		}
		this.time = end;
	}

	private due(end: number) {
		const ready = [...this.timers].filter(([, timer]) => timer.at <= end);
		return ready.sort((a, b) => a[1].at - b[1].at)[0];
	}

	private add(run: () => void, ms: number, every: number): number {
		const id = this.nextId++;
		this.timers.set(id, { run, at: this.time + ms, every });
		return id;
	}
}

/**
 * One end of a fake connection between a session and a `FakeRoom`. Frames wait in queues until a
 * test moves them. The link opens as soon as the session listens, so HELLO is the first upstream
 * frame, and the room sends INIT once HELLO is delivered.
 */
export class FakeLink implements SessionSocket {
	upstream: Uint8Array[] = [];
	downstream: Uint8Array[] = [];
	clientId = 0;
	closed = false;
	onmessage: SessionSocket['onmessage'] = null;
	onclose: SessionSocket['onclose'] = null;
	private listener: SessionSocket['onopen'] = null;

	constructor(
		private readonly room: FakeRoom,
		private readonly role: RoleValue = Role.Editor
	) {}

	get onopen(): SessionSocket['onopen'] {
		return this.listener;
	}

	set onopen(listener: SessionSocket['onopen']) {
		this.listener = listener;
		listener?.();
	}

	send(data: Uint8Array): void {
		if (!this.closed) this.upstream.push(data);
	}

	close(code = 1000): void {
		this.drop(code);
	}

	/** Hands the oldest upstream frame to the room. */
	deliver(): void {
		const frame = this.upstream.shift();
		if (!frame || this.closed) return;
		if (frame[0] !== FrameType.Hello) return this.room.receive(this.clientId, frame);
		this.clientId = this.room.join((data) => this.downstream.push(data), frame.slice(1), this.role);
	}

	/** Hands the oldest downstream frame to the session. */
	receive(): void {
		const frame = this.downstream.shift();
		if (frame && !this.closed) this.onmessage?.({ data: frame.slice().buffer });
	}

	/** Cuts the connection: in-flight frames are lost both ways, and the session hears `code`. */
	drop(code = 1006): void {
		if (this.closed) return;
		this.closed = true;
		this.upstream = [];
		this.downstream = [];
		if (this.clientId) this.room.leave(this.clientId);
		this.onclose?.({ code });
	}
}

/** The `client_seq` of every PUSH among `frames`. */
export const pushSeqs = (frames: Uint8Array[]): number[] =>
	frames.filter((frame) => frame[0] === FrameType.Push).map((frame) => new DataView(frame.buffer).getUint32(1));

/** How a test peer is set up: its randomness, HELLO origin and role. */
export type PeerOptions = { random?: () => number; origin?: Uint8Array; role?: RoleValue };

/** A session wired to a room through fake links, with a manual clock. */
export class Peer {
	links: FakeLink[] = [];
	readonly clock: ManualClock;
	readonly session: Session;

	constructor(room: FakeRoom, load: (bytes: Uint8Array) => Model, options: PeerOptions = {}) {
		this.clock = new ManualClock(options.random);
		this.session = new Session({
			url: 'ws://test/api/workbooks/1/ws',
			load,
			origin: options.origin ?? fresh(),
			clock: this.clock,
			open: () => {
				const link = new FakeLink(room, options.role);
				this.links.push(link);
				return link;
			}
		});
	}

	/** The live link. */
	get link(): FakeLink {
		return this.links[this.links.length - 1];
	}

	/** Delivers HELLO and hands INIT to the session: the link is joined and open. */
	join(): void {
		this.link.deliver();
		this.link.receive();
	}

	/** Moves frames both ways until neither queue holds one. */
	settle(): void {
		while (this.busy) {
			this.link.deliver();
			this.link.receive();
		}
	}

	/** Whether any frame is still waiting either way. */
	get busy(): boolean {
		return !this.link.closed && (this.link.upstream.length > 0 || this.link.downstream.length > 0);
	}
}
