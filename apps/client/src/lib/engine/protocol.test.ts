import { describe, expect, it } from 'vitest';
import { decodeServerFrame, encodeCursor, encodeHello, encodePush, ProtocolError, Role } from './protocol';
import { decodeClientFrame, encodeInit, encodePresence, encodeReject, encodeUpdate } from './protocol-server';

const bytes = (...values: number[]) => new Uint8Array(values);
const ascii = (text: string) => [...text].map((c) => c.charCodeAt(0));

describe('client frames, golden bytes from docs/api.md', () => {
	it('HELLO is 0x03 then the 16-byte origin', () => {
		const origin = Uint8Array.from({ length: 16 }, (_, i) => i + 1);
		expect(Array.from(encodeHello(origin))).toEqual([0x03, ...origin]);
		expect(decodeClientFrame(encodeHello(origin))).toEqual({ type: 'hello', origin });
	});

	it('HELLO refuses an origin that is not 16 bytes', () => {
		expect(() => encodeHello(new Uint8Array(15))).toThrow(ProtocolError);
		expect(() => decodeClientFrame(bytes(0x03, 1, 2))).toThrow(ProtocolError);
	});

	it('PUSH is 0x01, client_seq u32 BE, then the payload', () => {
		const frame = encodePush(0x01020304, bytes(0xaa, 0xbb));
		expect(Array.from(frame)).toEqual([0x01, 0x01, 0x02, 0x03, 0x04, 0xaa, 0xbb]);
	});

	it('PUSH with an empty payload still carries the header', () => {
		expect(Array.from(encodePush(7, bytes()))).toEqual([0x01, 0, 0, 0, 7]);
	});

	it('CURSOR is 0x02 then UTF-8 JSON with keys in the plan order', () => {
		const frame = encodeCursor({ sheet: 0, row: 1, column: 2, range: [1, 2, 3, 4] });
		const json = '{"sheet":0,"row":1,"column":2,"range":[1,2,3,4]}';
		expect(Array.from(frame)).toEqual([0x02, ...ascii(json)]);
	});

	it('round-trips through the server-side decoder', () => {
		const payload = bytes(9, 8, 7);
		expect(decodeClientFrame(encodePush(42, payload))).toEqual({ type: 'push', clientSeq: 42, payload });
	});
});

describe('server frames, golden bytes from docs/api.md', () => {
	const init = [0x10, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 7, 0x02, 0, 0, 0x01, 0x05, 0xde, 0xad];
	const update = [0x11, 0, 0, 0, 0, 0, 0, 0x01, 0x00, 0, 0, 0, 0, 0, 0, 0, 5, 0x42];

	it('INIT is seq u64, client_id u32, role u8, acked u32, snapshot', () => {
		const head = { seq: 2 ** 32 + 2, clientId: 7, role: Role.Editor, acked: 261 };
		expect(decodeServerFrame(bytes(...init))).toEqual({ type: 'init', ...head, snapshot: bytes(0xde, 0xad) });
		expect(Array.from(encodeInit(head, bytes(0xde, 0xad)))).toEqual(init);
	});

	it('INIT with no snapshot bytes still decodes', () => {
		expect(decodeServerFrame(bytes(...init.slice(0, 18)))).toMatchObject({ acked: 261, snapshot: bytes() });
	});

	it('UPDATE is seq u64, client_id u32 (0 = server), client_seq u32, payload', () => {
		const decoded = decodeServerFrame(bytes(...update));
		expect(decoded).toEqual({ type: 'update', seq: 256, clientId: 0, clientSeq: 5, payload: bytes(0x42) });
		expect(Array.from(encodeUpdate(256, 0, 5, bytes(0x42)))).toEqual(update);
	});

	it('REJECT is client_seq u32 then a UTF-8 reason', () => {
		const frame = [0x12, 0, 0, 0, 3, ...ascii('rate_limited')];
		expect(decodeServerFrame(bytes(...frame))).toEqual({ type: 'reject', clientSeq: 3, reason: 'rate_limited' });
		expect(Array.from(encodeReject(3, 'rate_limited'))).toEqual(frame);
	});

	it('REJECT may carry an empty reason', () => {
		expect(decodeServerFrame(bytes(0x12, 0, 0, 0, 1))).toEqual({ type: 'reject', clientSeq: 1, reason: '' });
	});

	it('PRESENCE is UTF-8 JSON, multi-byte names included', () => {
		const entries = [{ client_id: 7, name: 'Zoé', picture: '', cursor: null }];
		const frame = encodePresence(entries);
		expect(frame[0]).toBe(0x13);
		expect(new TextDecoder().decode(frame.subarray(1))).toBe('[{"client_id":7,"name":"Zoé","picture":"","cursor":null}]');
		expect(decodeServerFrame(frame)).toEqual({ type: 'presence', entries });
	});

	it('decodes a payload view that does not start at offset 0', () => {
		const backing = new Uint8Array([0xff, ...update]);
		expect(decodeServerFrame(backing.subarray(1))).toMatchObject({ seq: 256, clientSeq: 5 });
	});
});

describe('malformed frames', () => {
	it.each([
		['truncated INIT', [0x10, 0, 0]],
		['INIT without acked', [0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0]],
		['truncated UPDATE', [0x11, 0, 0, 0, 0, 0, 0, 0, 1]],
		['truncated REJECT', [0x12, 0]],
		['unknown type', [0x7f]],
		['empty frame', []],
		['unknown role', [0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 9, 0, 0, 0, 0]],
		['bad presence JSON', [0x13, ...ascii('[{')]],
		['seq past 2^53', [0x11, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]]
	])('%s throws ProtocolError', (_name, frame) => {
		expect(() => decodeServerFrame(bytes(...frame))).toThrow(ProtocolError);
	});
});
