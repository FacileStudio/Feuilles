import type { Model } from '@ironcalc/wasm';
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

/** One end of a fake connection between a session and a `FakeRoom`. Frames wait in queues until a test moves them. */
export class FakeLink implements SessionSocket {
	upstream: Uint8Array[] = [];
	downstream: Uint8Array[] = [];
	clientId = 0;
	closed = false;
	onmessage: SessionSocket['onmessage'] = null;
	onclose: SessionSocket['onclose'] = null;

	constructor(private readonly room: FakeRoom) {
		this.clientId = room.join((frame) => this.downstream.push(frame));
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
		if (frame && !this.closed) this.room.receive(this.clientId, frame);
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
		this.room.leave(this.clientId);
		this.onclose?.({ code });
	}
}

/** A session wired to a room through fake links, with a manual clock. */
export class Peer {
	links: FakeLink[] = [];
	readonly clock: ManualClock;
	readonly session: Session;

	constructor(room: FakeRoom, load: (bytes: Uint8Array) => Model, random?: () => number) {
		this.clock = new ManualClock(random);
		this.session = new Session({
			url: 'ws://test/api/workbooks/1/ws',
			load,
			clock: this.clock,
			open: () => {
				const link = new FakeLink(room);
				this.links.push(link);
				return link;
			}
		});
	}

	/** The live link. */
	get link(): FakeLink {
		return this.links[this.links.length - 1];
	}

	/** Whether any frame is still waiting either way. */
	get busy(): boolean {
		return !this.link.closed && (this.link.upstream.length > 0 || this.link.downstream.length > 0);
	}
}
