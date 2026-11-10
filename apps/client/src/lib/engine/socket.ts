/** The part of a WebSocket a session uses, so tests can drive one by hand. */
export interface SessionSocket {
	send(data: Uint8Array<ArrayBuffer>): void;
	close(code?: number): void;
	onmessage: ((event: { data: unknown }) => void) | null;
	onclose: ((event: { code: number }) => void) | null;
}

/** Time and randomness, injected so tests are deterministic. */
export interface Clock {
	now(): number;
	random(): number;
	setTimeout(run: () => void, ms: number): unknown;
	clearTimeout(id: unknown): void;
	setInterval(run: () => void, ms: number): unknown;
	clearInterval(id: unknown): void;
}

/** The real clock. */
export const defaultClock: Clock = {
	now: () => Date.now(),
	random: () => Math.random(),
	setTimeout: (run, ms) => globalThis.setTimeout(run, ms),
	clearTimeout: (id) => globalThis.clearTimeout(id as ReturnType<typeof setTimeout>),
	setInterval: (run, ms) => globalThis.setInterval(run, ms),
	clearInterval: (id) => globalThis.clearInterval(id as ReturnType<typeof setInterval>)
};

/** Turns a same-origin path into a ws:// or wss:// URL. */
export function socketUrl(path: string, location: { protocol: string; host: string }): string {
	const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
	return `${scheme}://${location.host}${path}`;
}

/** Opens a binary WebSocket behind the `SessionSocket` shape. */
export function browserSocket(url: string): SessionSocket {
	const ws = new WebSocket(url);
	ws.binaryType = 'arraybuffer';
	const socket: SessionSocket = {
		send: (data) => ws.send(data),
		close: (code) => ws.close(code),
		onmessage: null,
		onclose: null
	};
	ws.onmessage = (event) => socket.onmessage?.(event);
	ws.onclose = (event) => socket.onclose?.(event);
	return socket;
}
