/** The two colours a theme paints an unstyled cell with, as hex. */
export type Ink = { text: string; background: string };

const MIN_CONTRAST = 3;
const AUTOMATIC_TEXT = 0.02;
const AUTOMATIC_FILL = 0.9;

/** WCAG relative luminance of a `#rgb`, `#rrggbb` or `#rrggbbaa` colour, or null when it is not hex. */
export function luminance(color: string): number | null {
	const hex = color.trim().replace(/^#/, '');
	const full = hex.length === 3 ? [...hex].map((c) => c + c).join('') : hex.slice(0, 6);
	if (!/^[0-9a-f]{6}$/i.test(full)) return null;
	const [r, g, b] = [0, 2, 4].map((i) => {
		const c = Number.parseInt(full.slice(i, i + 2), 16) / 255;
		return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
	});
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio between two hex colours, 1 when either cannot be read. */
export function contrast(a: string, b: string): number {
	const la = luminance(a);
	const lb = luminance(b);
	if (la === null || lb === null) return 1;
	return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/**
 * The fill a cell shows, or null for the theme background. White and near-white fills are
 * spreadsheet "automatic" fills (xlsx files and IronCalc write them for unfilled cells), so they
 * follow the theme instead of glaring in dark mode.
 */
export function cellFill(fill: string | null | undefined): string | null {
	if (!fill) return null;
	const l = luminance(fill);
	return l !== null && l >= AUTOMATIC_FILL ? null : fill;
}

/** A border or text colour: missing and black or near-black colours are "automatic" and become `text`. */
export function lineInk(color: string | null | undefined, text: string): string {
	const l = color ? luminance(color) : null;
	return !color || (l !== null && l <= AUTOMATIC_TEXT) ? text : color;
}

/**
 * The colour cell text drawn in `color` takes on `background`: automatic colours follow the theme
 * (see `lineInk`), and any colour left under 3:1 falls back to whichever theme colour reads best.
 */
export function cellInk(color: string | null | undefined, background: string, ink: Ink): string {
	const chosen = lineInk(color, ink.text);
	if (luminance(chosen) === null || contrast(chosen, background) >= MIN_CONTRAST) return chosen;
	return contrast(ink.text, background) >= contrast(ink.background, background) ? ink.text : ink.background;
}
