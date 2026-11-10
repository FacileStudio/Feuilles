import { FrameType, ORIGIN_BYTES, type ClientFrame, type Cursor, type PresenceEntry, type RoleValue } from './protocol';
import { frame, need, parseJson, ProtocolError, utf8 } from './wire';

/** The head of an INIT frame. `acked` is the highest `client_seq` accepted from the socket's origin. */
export type InitHead = { seq: number; clientId: number; role: RoleValue; acked: number };

/** Encodes INIT: `0x10`, `seq: u64`, `client_id: u32`, `role: u8`, `acked: u32`, snapshot. */
export function encodeInit(head: InitHead, snapshot: Uint8Array): Uint8Array<ArrayBuffer> {
	const { bytes, view } = frame(FrameType.Init, 17, snapshot);
	view.setBigUint64(1, BigInt(head.seq));
	view.setUint32(9, head.clientId);
	view.setUint8(13, head.role);
	view.setUint32(14, head.acked);
	return bytes;
}

/** Encodes UPDATE: `0x11`, `seq: u64`, `client_id: u32`, `client_seq: u32`, payload. */
export function encodeUpdate(seq: number, clientId: number, clientSeq: number, payload: Uint8Array): Uint8Array<ArrayBuffer> {
	const { bytes, view } = frame(FrameType.Update, 16, payload);
	view.setBigUint64(1, BigInt(seq));
	view.setUint32(9, clientId);
	view.setUint32(13, clientSeq);
	return bytes;
}

/** Encodes REJECT: `0x12`, `client_seq: u32`, UTF-8 reason. */
export function encodeReject(clientSeq: number, reason: string): Uint8Array<ArrayBuffer> {
	const { bytes, view } = frame(FrameType.Reject, 4, utf8(reason));
	view.setUint32(1, clientSeq);
	return bytes;
}

/** Encodes PRESENCE: `0x13`, UTF-8 JSON list. */
export function encodePresence(entries: PresenceEntry[]): Uint8Array<ArrayBuffer> {
	return frame(FrameType.Presence, 0, utf8(JSON.stringify(entries))).bytes;
}

/** Decodes one client-to-server frame. The dev mock and the tests use it. */
export function decodeClientFrame(bytes: Uint8Array): ClientFrame {
	switch (bytes[0]) {
		case FrameType.Push: {
			const view = need(bytes, 5, 'PUSH');
			return { type: 'push', clientSeq: view.getUint32(1), payload: bytes.slice(5) };
		}
		case FrameType.Cursor:
			return { type: 'cursor', cursor: parseJson<Cursor>(bytes.subarray(1), 'CURSOR') };
		case FrameType.Hello:
			if (bytes.length !== 1 + ORIGIN_BYTES) throw new ProtocolError('HELLO carries exactly 16 bytes');
			return { type: 'hello', origin: bytes.slice(1) };
		default:
			throw new ProtocolError(`unknown client frame type ${bytes[0]}`);
	}
}
