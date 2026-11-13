import { ApiError } from './api/error';
import type { PendingAction, ServerVersion, SessionAnswer, Space, Workbook } from './api/types';

export { ApiError } from './api/error';
export type * from './api/types';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const safe = new Set(['GET', 'HEAD', 'OPTIONS']);
const json = { 'Content-Type': 'application/json' };

/** Builds the API client over a `fetch`. Every path is relative to the page's origin. */
export function createBackend(fetcher: Fetch) {
	async function call<T>(path: string, init: RequestInit = {}): Promise<T> {
		const headers = new Headers(init.headers);
		if (!safe.has((init.method ?? 'GET').toUpperCase())) headers.set('X-Facile-CSRF', '1');
		let response: Response;
		try {
			response = await fetcher(path, { ...init, headers, credentials: 'include' });
		} catch {
			throw new ApiError(0, 'network', 'The server could not be reached');
		}
		if (!response.ok) throw await ApiError.from(response);
		return (response.status === 204 ? undefined : await response.json()) as T;
	}

	const send = <T>(method: string, path: string, body?: unknown) =>
		call<T>(path, { method, headers: json, body: body === undefined ? undefined : JSON.stringify(body) });

	return {
		session: () => call<SessionAnswer>('/api/auth/session'),
		logout: () => send<{ logged_out: boolean }>('POST', '/api/auth/logout'),
		syncProfile: () => send<{ synced: boolean }>('POST', '/api/auth/sync-profile').then((r) => r.synced),
		version: () => call<ServerVersion>('/version'),
		spaces: () => call<{ spaces: Space[] }>('/api/spaces').then((r) => r.spaces),
		list: (spaceId: string | null, trashed = false) =>
			call<{ workbooks: Workbook[] }>(`/api/workbooks?${scope(spaceId, { trashed: String(trashed) })}`).then((r) => r.workbooks),
		create: (title: string, spaceId: string | null) =>
			send<Workbook>('POST', '/api/workbooks', spaceId ? { title, space_id: spaceId } : { title }),
		import: (file: File, spaceId: string | null) => call<Workbook>('/api/workbooks/import', { method: 'POST', body: upload(file, spaceId) }),
		get: (id: string) => call<Workbook>(`/api/workbooks/${encodeURIComponent(id)}`),
		rename: (id: string, title: string) => send<Workbook>('PATCH', `/api/workbooks/${encodeURIComponent(id)}`, { title }),
		trash: (id: string) => send<void>('DELETE', `/api/workbooks/${encodeURIComponent(id)}`),
		restore: (id: string) => send<Workbook>('POST', `/api/workbooks/${encodeURIComponent(id)}/restore`),
		destroy: (id: string) => send<void>('DELETE', `/api/workbooks/${encodeURIComponent(id)}/permanent`),
		actions: () => call<PendingAction[]>('/api/mcp/actions'),
		confirmAction: (id: string) => send<{ status: string }>('POST', `/api/mcp/actions/${encodeURIComponent(id)}/confirm`),
		dismissAction: (id: string) => send<void>('POST', `/api/mcp/actions/${encodeURIComponent(id)}/dismiss`)
	};
}

/** The query string of a list call: the active space, when there is one, then `extra`. */
export function scope(spaceId: string | null, extra: Record<string, string> = {}): string {
	return new URLSearchParams({ ...(spaceId ? { space_id: spaceId } : {}), ...extra }).toString();
}

/** The multipart body of `POST /api/workbooks/import`. */
export function upload(file: File, spaceId: string | null): FormData {
	const form = new FormData();
	form.append('file', file);
	if (spaceId) form.append('space_id', spaceId);
	return form;
}

/** Where the browser goes to sign in through Registre. */
export const signInUrl = '/api/auth/oidc';

/** The download address of an export. */
export function exportUrl(id: string, format: 'xlsx' | 'csv', sheet = 0): string {
	const base = `/api/workbooks/${encodeURIComponent(id)}/export.${format}`;
	return format === 'csv' ? `${base}?sheet=${sheet}` : base;
}

/** The WebSocket path of a workbook (plan 7.2). */
export const socketPath = (id: string) => `/api/workbooks/${encodeURIComponent(id)}/ws`;

/** The API client the app uses. */
export const backend = createBackend((input, init) => fetch(input, init));
