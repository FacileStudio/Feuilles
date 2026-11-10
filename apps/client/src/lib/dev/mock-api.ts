import type { ServerWebSocket } from 'bun';
import type { Workbook } from '../api/types';
import { decodeClientFrame, encodePresence } from '../engine/protocol-server';
import type { Cursor } from '../engine/protocol';
import { emptySnapshot, load, loadEngineSync } from './engine-node';
import { FakeRoom } from './room';

type Socket = ServerWebSocket<{ workbook: string; clientId: number; name: string }>;

const port = Number(process.env.MOCK_PORT ?? 4000);
const workbooks = new Map<string, Workbook>();
const rooms = new Map<string, { room: FakeRoom; people: Map<number, { socket: Socket; cursor: Cursor | null }> }>();
let guests = 0;

loadEngineSync();

const json = (body: unknown, status = 200) => Response.json(body, { status });
const fail = (status: number, code: string, message: string) => json({ error: { code, message } }, status);

function newWorkbook(title: string): Workbook {
	const now = new Date().toISOString();
	const workbook: Workbook = {
		id: crypto.randomUUID(),
		space_id: null,
		title,
		locale: 'en',
		timezone: 'UTC',
		language: 'en',
		sheets: [{ name: 'Sheet1' }],
		created_at: now,
		updated_at: now,
		trashed_at: null,
		engine_version: '0.9.1'
	};
	workbooks.set(workbook.id, workbook);
	return workbook;
}

function roomFor(id: string) {
	let entry = rooms.get(id);
	if (!entry) rooms.set(id, (entry = { room: new FakeRoom(load, emptySnapshot()), people: new Map() }));
	return entry;
}

function broadcastPresence(id: string): void {
	const { people } = roomFor(id);
	const list = [...people.entries()].map(([clientId, p]) => ({ client_id: clientId, name: p.socket.data.name, picture: '', cursor: p.cursor }));
	for (const person of people.values()) person.socket.send(encodePresence(list));
}

async function workbookRoute(request: Request, id: string, action: string | undefined): Promise<Response> {
	const workbook = workbooks.get(id);
	if (!workbook) return fail(404, 'not_found', 'workbook not found');
	if (request.method === 'GET' && !action) return json(workbook);
	if (request.method === 'PATCH') Object.assign(workbook, { title: ((await request.json()) as { title: string }).title });
	else if (request.method === 'DELETE' && !action) workbook.trashed_at = new Date().toISOString();
	else if (action === 'restore') workbook.trashed_at = null;
	else if (action === 'permanent') workbooks.delete(id);
	else return fail(405, 'method_not_allowed', 'not in the mock');
	return request.method === 'DELETE' ? new Response(null, { status: 204 }) : json(workbook);
}

async function route(request: Request, url: URL): Promise<Response> {
	const path = url.pathname;
	if (path === '/version') return json({ version: 'mock', engine: '0.9.1' });
	if (path === '/api/auth/session') return json({ user: { email: 'mock@facile.studio', name: 'Mock person' } });
	if (path === '/api/spaces') return json({ spaces: [] });
	if (path === '/api/workbooks' && request.method === 'GET') {
		const trashed = url.searchParams.get('trashed') === 'true';
		return json({ workbooks: [...workbooks.values()].filter((w) => (w.trashed_at !== null) === trashed) });
	}
	if (path === '/api/workbooks' && request.method === 'POST') {
		return json(newWorkbook(((await request.json()) as { title: string }).title), 201);
	}
	const match = path.match(/^\/api\/workbooks\/([^/]+)(?:\/([a-z]+))?$/);
	return match ? workbookRoute(request, match[1], match[2]) : fail(404, 'not_found', 'no such route in the mock');
}

Bun.serve<{ workbook: string; clientId: number; name: string }>({
	port,
	hostname: '127.0.0.1',
	fetch(request, server) {
		const url = new URL(request.url);
		const ws = url.pathname.match(/^\/api\/workbooks\/([^/]+)\/ws$/);
		if (ws && workbooks.has(ws[1])) {
			const ok = server.upgrade(request, { data: { workbook: ws[1], clientId: 0, name: `Guest ${++guests}` } });
			return ok ? undefined : fail(400, 'invalid_argument', 'upgrade failed');
		}
		return route(request, url);
	},
	websocket: {
		open(socket: Socket) {
			const entry = roomFor(socket.data.workbook);
			socket.data.clientId = entry.room.join((frame) => socket.send(frame));
			entry.people.set(socket.data.clientId, { socket, cursor: null });
			broadcastPresence(socket.data.workbook);
		},
		message(socket: Socket, message) {
			const bytes = typeof message === 'string' ? new TextEncoder().encode(message) : new Uint8Array(message);
			const entry = roomFor(socket.data.workbook);
			const frame = decodeClientFrame(bytes);
			if (frame.type === 'push') return entry.room.receive(socket.data.clientId, bytes);
			const person = entry.people.get(socket.data.clientId);
			if (person) person.cursor = frame.cursor;
			broadcastPresence(socket.data.workbook);
		},
		close(socket: Socket) {
			const entry = roomFor(socket.data.workbook);
			entry.room.leave(socket.data.clientId);
			entry.people.delete(socket.data.clientId);
			broadcastPresence(socket.data.workbook);
		}
	}
});

console.log(`feuilles mock api on http://127.0.0.1:${port}`);
