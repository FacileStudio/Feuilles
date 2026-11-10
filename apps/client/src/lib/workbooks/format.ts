const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
const steps: [Intl.RelativeTimeFormatUnit, number][] = [
	['year', 31_536_000],
	['month', 2_592_000],
	['week', 604_800],
	['day', 86_400],
	['hour', 3_600],
	['minute', 60]
];

/** "3 minutes ago", "yesterday": how long ago an ISO date was, relative to `now`. */
export function ago(iso: string, now = Date.now()): string {
	const seconds = Math.round((Date.parse(iso) - now) / 1000);
	for (const [unit, size] of steps) {
		if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit);
	}
	return 'just now';
}

/** Trims a title and checks the API's 1 to 200 character rule (plan 7.5). Returns null when it fails. */
export function cleanTitle(raw: string): string | null {
	const title = raw.trim();
	return title.length >= 1 && [...title].length <= 200 ? title : null;
}

/** A title for an imported file: its name without the extension. */
export function titleFromFile(name: string): string {
	return cleanTitle(name.replace(/\.[^.]+$/, '')) ?? 'Imported workbook';
}
