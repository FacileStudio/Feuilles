import type { Model } from '@ironcalc/wasm';
import {
	decodeServerFrame,
	encodeCursor,
	encodePush,
	Role,
	type Cursor,
	type PresenceEntry,
	type RoleValue,
	type ServerFrame
} from './protocol';
import { Replica, type Pending } from './replica';
import { browserSocket, defaultClock, type Clock, type SessionSocket } from './socket';

/** Where a session stands. `revoked` means the server removed this person from the workbook. */
export type SessionStatus = 'connecting' | 'open' | 'reconnecting' | 'revoked' | 'closed';

/** How a session reaches the outside world. Tests replace every part. */
export type SessionOptions = {
	url: string;
	load: (bytes: Uint8Array) => Model;
	open?: (url: string) => SessionSocket;
	clock?: Clock;
};

/** Largest PUSH payload the server takes (plan 7.2). A larger one would close the socket with 1009. */
export const MAX_PUSH_BYTES = 1024 * 1024;
const KEEPALIVE_MS = 25_000;
const CURSOR_MS = 100;
const POLICY_VIOLATION = 1008;
const homeCursor: Cursor = { sheet: 0, row: 1, column: 1, range: [1, 1, 1, 1] };

/**
 * One open workbook: the socket, the replica of plan 7.1, reconnect with resend, and presence.
 * `revision` changes whenever `view` does, so the grid knows when to redraw.
 */
export class Session {
	status = $state<SessionStatus>('connecting');
	role = $state<RoleValue | null>(null);
	presence = $state.raw<PresenceEntry[]>([]);
	revision = $state(0);
	notice = $state<string | null>(null);
	replica = $state.raw<Replica<Model> | null>(null);
	private socket: SessionSocket | null = null;
	private attempts = 0;
	private retry: unknown = null;
	private keepalive: unknown = null;
	private cursorTimer: unknown = null;
	private cursor: Cursor = homeCursor;
	private cursorSentAt = -Infinity;
	private readonly open: (url: string) => SessionSocket;
	private readonly clock: Clock;

	constructor(private readonly options: SessionOptions) {
		this.open = options.open ?? browserSocket;
		this.clock = options.clock ?? defaultClock;
		this.connect();
	}

	/** The model the grid draws, or null before the first INIT. */
	get view(): Model | null {
		return this.replica?.view ?? null;
	}

	/** Whether this person may edit. */
	get canEdit(): boolean {
		return this.role === Role.Editor && this.replica !== null && this.status !== 'revoked';
	}

	/** The client id INIT gave this connection. */
	get clientId(): number | null {
		return this.replica?.clientId ?? null;
	}

	/** Runs a local edit on `view` and pushes the batch it produced. Returns false when editing is not allowed. */
	edit(change: (model: Model) => void): boolean {
		if (!this.replica) return false;
		if (!this.canEdit) {
			this.replica.discard();
			this.revision += 1;
			return false;
		}
		const entry = this.replica.edit(change);
		if (entry && entry.batch.length > MAX_PUSH_BYTES) {
			this.replica.reject(entry.clientSeq);
			this.notice = 'too_large';
		} else if (entry) {
			this.push(entry);
		}
		this.revision += 1;
		return true;
	}

	/** Records the local selection and sends it, at most ten times a second. */
	setCursor(cursor: Cursor): void {
		this.cursor = cursor;
		if (this.cursorTimer !== null) return;
		const wait = this.cursorSentAt + CURSOR_MS - this.clock.now();
		if (wait <= 0) return this.sendCursor();
		this.cursorTimer = this.clock.setTimeout(() => {
			this.cursorTimer = null;
			this.sendCursor();
		}, wait);
	}

	/** Marks a view change made outside `edit`, such as a selection, so the grid redraws. */
	touch(): void {
		this.revision += 1;
	}

	/** Closes the socket for good and frees the WASM models. */
	close(): void {
		this.status = 'closed';
		this.stopTimers();
		this.socket?.close(1000);
		this.socket = null;
		this.replica?.dispose();
		this.replica = null;
	}

	private connect(): void {
		const socket = this.open(this.options.url);
		this.socket = socket;
		socket.onmessage = (event) => {
			if (event.data instanceof ArrayBuffer) this.receive(new Uint8Array(event.data));
		};
		socket.onclose = (event) => this.closed(socket, event.code);
	}

	private receive(bytes: Uint8Array): void {
		let frame: ServerFrame;
		try {
			frame = decodeServerFrame(bytes);
		} catch {
			return this.resync();
		}
		if (frame.type === 'init') return this.init(frame);
		if (!this.replica || this.status !== 'open') return;
		if (frame.type === 'presence') this.presence = frame.entries.filter((entry) => entry.client_id !== this.clientId);
		else if (frame.type === 'reject') this.refused(frame.clientSeq, frame.reason);
		else if (this.replica.update(frame) === 'desync') this.resync();
		else this.revision += 1;
	}

	private init(frame: Extract<ServerFrame, { type: 'init' }>): void {
		const resend = this.replica
			? this.replica.reset(frame.snapshot, frame.seq, frame.clientId)
			: this.first(frame);
		this.role = frame.role;
		this.status = 'open';
		this.attempts = 0;
		for (const entry of resend) this.push(entry);
		this.sendCursor();
		this.clock.clearInterval(this.keepalive);
		this.keepalive = this.clock.setInterval(() => this.sendCursor(), KEEPALIVE_MS);
		this.revision += 1;
	}

	private first(frame: Extract<ServerFrame, { type: 'init' }>): Pending[] {
		this.replica = new Replica(this.options.load, frame.snapshot, frame.seq, frame.clientId);
		return [];
	}

	private refused(clientSeq: number, reason: string): void {
		this.replica?.reject(clientSeq);
		this.notice = reason || 'refused';
		this.revision += 1;
	}

	private push(entry: Pending): void {
		if (this.status === 'open') this.socket?.send(encodePush(entry.clientSeq, entry.batch));
	}

	private sendCursor(): void {
		if (this.status !== 'open') return;
		this.cursorSentAt = this.clock.now();
		this.socket?.send(encodeCursor(this.cursor));
	}

	private resync(): void {
		this.socket?.close(1000);
	}

	private closed(socket: SessionSocket, code: number): void {
		if (socket !== this.socket || this.status === 'closed') return;
		this.stopTimers();
		this.socket = null;
		this.presence = [];
		if (code === POLICY_VIOLATION) {
			this.status = 'revoked';
			return;
		}
		this.status = 'reconnecting';
		const delay = Math.min(10_000, 500 * 2 ** this.attempts) * (0.5 + this.clock.random() / 2);
		this.attempts += 1;
		this.retry = this.clock.setTimeout(() => this.connect(), delay);
	}

	private stopTimers(): void {
		this.clock.clearTimeout(this.retry);
		this.clock.clearTimeout(this.cursorTimer);
		this.clock.clearInterval(this.keepalive);
		this.retry = this.cursorTimer = this.keepalive = null;
	}
}
