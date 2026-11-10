import { backend, type Space } from '$lib/backend';
import { readPref, writePref } from './storage';

const KEY = 'feuilles.space';

/** The caller's spaces and the active one. `currentId` null is the personal scope. */
export const spaces = $state<{ list: Space[]; currentId: string | null; loaded: boolean }>({
	list: [],
	currentId: null,
	loaded: false
});

/** Loads the spaces and restores the last active one when it still exists. */
export async function loadSpaces(): Promise<void> {
	spaces.list = await backend.spaces().catch(() => []);
	const stored = readPref(KEY);
	spaces.currentId = spaces.list.some((space) => space.id === stored) ? stored : null;
	spaces.loaded = true;
}

/** Switches the active space. */
export function selectSpace(id: string | null): void {
	spaces.currentId = id;
	writePref(KEY, id);
}
