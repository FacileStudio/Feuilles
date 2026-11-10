import { applyTheme, isTheme, type Theme } from '@facile/muse';
import { readPref, writePref } from './storage';

const KEY = 'feuilles.theme';

/** The theme picked in this browser. */
export const theme = $state<{ mode: Theme }>({ mode: 'system' });

/** Applies the stored theme at start-up. */
export function restoreTheme(): void {
	const stored = readPref(KEY);
	theme.mode = isTheme(stored) ? stored : 'system';
	applyTheme(theme.mode);
}

/** Picks and stores a theme. */
export function setTheme(mode: Theme): void {
	theme.mode = mode;
	applyTheme(mode);
	writePref(KEY, mode);
}
