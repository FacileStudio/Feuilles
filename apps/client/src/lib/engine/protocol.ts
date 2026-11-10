import { frame, fromUtf8, need, parseJson, ProtocolError, readU64, utf8 } from './wire';

export { ProtocolError } from './wire';

/** Frame type bytes of the Feuilles wire protocol (plan section 7.2). */
export const FrameType = {
	Push: 0x01,
	Cursor: 0x02,
	Init: 0x10,
	Update: 0x11,
	Reject: 0x12,
	Presence: 0x13
} as const;

/** Role byte of an INIT frame. */
export const Role = { Viewer: 1, Editor: 2 } as const;

/** A role as INIT announces it. */
export type RoleValue = (typeof Role)[keyof typeof Role];

/** A selection as CURSOR carries it: the active cell and the selected range `[r1, c1, r2, c2]`. */
export type Cursor = {
	sheet: number;
	row: number;
	column: number;
	range: [number, number, number, number];
};

/** One collaborator in a PRESENCE list. */
export type PresenceEntry = {
	client_id: number;
	name: string;
	picture: string;
	cursor: Cursor | null;
};

/** A frame the server sends. */
export type ServerFrame =
	| { type: 'init'; seq: number; clientId: number; role: RoleValue; snapshot: Uint8Array }
	| { type: 'update'; seq: number; clientId: number; clientSeq: number; payload: Uint8Array }
	| { type: 'reject'; clientSeq: number; reason: string }
	| { type: 'presence'; entries: PresenceEntry[] };

/** A frame the client sends. */
export type ClientFrame =
	| { type: 'push'; clientSeq: number; payload: Uint8Array }
	| { type: 'cursor'; cursor: Cursor };

/** Encodes PUSH: `0x01`, `client_seq: u32`, payload. */
export function encodePush(clientSeq: number, payload: Uint8Array): Uint8Array<ArrayBuffer> {
	const { bytes, view } = frame(FrameType.Push, 4, payload);
	view.setUint32(1, clientSeq);
	return bytes;
}

/** Encodes CURSOR: `0x02`, UTF-8 JSON of the selection. */
export function encodeCursor(cursor: Cursor): Uint8Array<ArrayBuffer> {
	const { sheet, row, column, range } = cursor;
	return frame(FrameType.Cursor, 0, utf8(JSON.stringify({ sheet, row, column, range }))).bytes;
}

/** Decodes one server-to-client frame. Throws `ProtocolError` on anything else. */
export function decodeServerFrame(bytes: Uint8Array): ServerFrame {
	switch (bytes[0]) {
		case FrameType.Init: {
			const view = need(bytes, 14, 'INIT');
			const role = view.getUint8(13);
			if (role !== Role.Viewer && role !== Role.Editor) throw new ProtocolError(`unknown role ${role}`);
			return { type: 'init', seq: readU64(view, 1), clientId: view.getUint32(9), role, snapshot: bytes.slice(14) };
		}
		case FrameType.Update: {
			const view = need(bytes, 17, 'UPDATE');
			const head = { seq: readU64(view, 1), clientId: view.getUint32(9), clientSeq: view.getUint32(13) };
			return { type: 'update', ...head, payload: bytes.slice(17) };
		}
		case FrameType.Reject: {
			const view = need(bytes, 5, 'REJECT');
			return { type: 'reject', clientSeq: view.getUint32(1), reason: fromUtf8(bytes.subarray(5)) };
		}
		case FrameType.Presence:
			return { type: 'presence', entries: parseJson<PresenceEntry[]>(bytes.subarray(1), 'PRESENCE') };
		default:
			throw new ProtocolError(`unknown server frame type ${bytes[0]}`);
	}
}
