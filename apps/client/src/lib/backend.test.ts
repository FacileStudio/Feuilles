import { describe, expect, it } from 'vitest';
import { ApiError, createBackend, exportUrl, scope } from './backend';

type Call = { input: string; init: RequestInit };

function fake(response: () => Response | Promise<Response>) {
	const calls: Call[] = [];
	const client = createBackend(async (input, init = {}) => {
		calls.push({ input, init });
		return response();
	});
	return { calls, client };
}

const ok = (body: unknown, status = 200) => () => new Response(JSON.stringify(body), { status });
const header = (call: Call, name: string) => new Headers(call.init.headers).get(name);

describe('backend', () => {
	it('sends credentials and no CSRF header on reads', async () => {
		const { calls, client } = fake(ok({ workbooks: [] }));
		await client.list('sp1');
		expect(calls[0].input).toBe('/api/workbooks?space_id=sp1&trashed=false');
		expect(calls[0].init.credentials).toBe('include');
		expect(header(calls[0], 'X-Facile-CSRF')).toBeNull();
	});

	it('sends the CSRF header and JSON on every mutation', async () => {
		const { calls, client } = fake(ok({ id: 'w1' }, 201));
		await client.create('Budget', null);
		await client.rename('w1', 'Budget 2');
		for (const call of calls) {
			expect(header(call, 'X-Facile-CSRF')).toBe('1');
			expect(header(call, 'Content-Type')).toBe('application/json');
		}
		expect(JSON.parse(String(calls[0].init.body))).toEqual({ title: 'Budget' });
		expect(calls[1].init.method).toBe('PATCH');
	});

	it('posts an import as multipart, leaving the boundary to fetch', async () => {
		const { calls, client } = fake(ok({ id: 'w1' }, 201));
		await client.import(new File(['x'], 'a.xlsx'), 'sp1');
		expect(calls[0].init.body).toBeInstanceOf(FormData);
		expect((calls[0].init.body as FormData).get('space_id')).toBe('sp1');
		expect(header(calls[0], 'Content-Type')).toBeNull();
		expect(header(calls[0], 'X-Facile-CSRF')).toBe('1');
	});

	it('turns the error envelope into a typed ApiError', async () => {
		const { client } = fake(ok({ error: { code: 'not_found', message: 'workbook not found' } }, 404));
		const error = await client.get('nope').catch((e: unknown) => e);
		expect(error).toBeInstanceOf(ApiError);
		expect(error).toMatchObject({ status: 404, code: 'not_found', message: 'workbook not found' });
	});

	it('falls back to the status when the body is not the envelope', async () => {
		const { client } = fake(() => new Response('<html>', { status: 429 }));
		await expect(client.spaces()).rejects.toMatchObject({ status: 429, code: 'rate_limited' });
	});

	it('reports a network failure as code network', async () => {
		const { client } = fake(() => Promise.reject(new TypeError('offline')));
		await expect(client.session()).rejects.toMatchObject({ status: 0, code: 'network' });
	});

	it('answers undefined on 204', async () => {
		const { client } = fake(() => new Response(null, { status: 204 }));
		await expect(client.trash('w1')).resolves.toBeUndefined();
	});

	it('unwraps the session user, null when signed out', async () => {
		const { client } = fake(ok({ user: null }));
		await expect(client.session()).resolves.toBeNull();
	});

	it('builds scope and export addresses', () => {
		expect(scope(null, { trashed: 'true' })).toBe('trashed=true');
		expect(exportUrl('w 1', 'csv', 2)).toBe('/api/workbooks/w%201/export.csv?sheet=2');
		expect(exportUrl('w1', 'xlsx')).toBe('/api/workbooks/w1/export.xlsx');
	});
});
