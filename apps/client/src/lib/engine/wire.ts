/** Thrown when bytes are not a valid frame. */
export class ProtocolError extends Error {}

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

/** Encodes a string as UTF-8. */
export const utf8 = (text: string): Uint8Array => encoder.encode(text);

/** Decodes UTF-8, throwing on invalid bytes. */
export const fromUtf8 = (bytes: Uint8Array): string => decoder.decode(bytes);

/** Allocates a frame: the type byte, `headerBytes` zeroed bytes, then `body`. */
export function frame(type: number, headerBytes: number, body: Uint8Array): { bytes: Uint8Array<ArrayBuffer>; view: DataView } {
	const bytes = new Uint8Array(1 + headerBytes + body.length);
	bytes[0] = type;
	bytes.set(body, 1 + headerBytes);
	return { bytes, view: new DataView(bytes.buffer) };
}

/** Reads a big-endian u64 as a number, refusing values past 2^53. */
export function readU64(view: DataView, offset: number): number {
	const value = view.getBigUint64(offset);
	if (value > BigInt(Number.MAX_SAFE_INTEGER)) throw new ProtocolError('seq exceeds the safe integer range');
	return Number(value);
}

/** Checks a frame holds at least `length` bytes and returns a view over it. */
export function need(bytes: Uint8Array, length: number, name: string): DataView {
	if (bytes.length < length) throw new ProtocolError(`${name} frame is truncated`);
	return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
}

/** Parses a UTF-8 JSON body. */
export function parseJson<T>(bytes: Uint8Array, name: string): T {
	try {
		return JSON.parse(fromUtf8(bytes)) as T;
	} catch {
		throw new ProtocolError(`${name} frame is not UTF-8 JSON`);
	}
}
