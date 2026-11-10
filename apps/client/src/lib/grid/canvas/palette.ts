import { USER_COLORS } from '@facile/muse';

/**
 * Stands in for upstream's `Editor/util.tsx` `getColor`, a React module the renderer only used
 * for its colour list. Formula references and collaborators' cursors take muse's identity palette.
 */
export function getColor(index: number): string {
	return USER_COLORS[Math.abs(index) % USER_COLORS.length];
}
