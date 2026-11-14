const KEY = 'feuilles:return-to';

/** `path` when it is a path on this origin to come back to after sign-in, otherwise null. */
export function safeReturn(path: string | null): string | null {
	if (!path || !path.startsWith('/') || path.startsWith('//') || path.startsWith('/\\')) return null;
	return path;
}

/** Keeps `path` for this tab until the next signed-in page load. Blocked storage forgets it. */
export function rememberReturn(path: string, storage: Storage | undefined = globalThis.sessionStorage): void {
	try {
		storage?.setItem(KEY, path);
	} catch {
		return;
	}
}

/** Reads and forgets the kept path, if it is safe to go to. */
export function takeReturn(storage: Storage | undefined = globalThis.sessionStorage): string | null {
	try {
		const path = storage?.getItem(KEY) ?? null;
		storage?.removeItem(KEY);
		return safeReturn(path);
	} catch {
		return null;
	}
}
