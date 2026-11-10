/** Reads a per-browser preference. Private windows and blocked storage read as unset. */
export function readPref(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}

/** Writes a per-browser preference; `null` removes it. Failures are ignored on purpose: it is a convenience. */
export function writePref(key: string, value: string | null): void {
	try {
		if (value === null) localStorage.removeItem(key);
		else localStorage.setItem(key, value);
	} catch {
		return;
	}
}
