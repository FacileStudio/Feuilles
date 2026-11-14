import { describe, expect, it } from 'vitest';
import { catalogs } from './messages';

const locales = ['fr', 'es', 'de'] as const;
const banned = [/ironcalc/i, /\bmcp\b/i, /du studio/i, /tels quels/i, /as they are/i, /self-hosted/i, /auto-hébergé/i, /votre serveur/i, /souverain/i];
const pitch = ['landing.slogan', 'landing.subtitle', 'landing.line', 'landing.title', 'landing.meta'];

function placeholders(message: unknown): string {
	const texts = typeof message === 'string' ? [message] : Object.values(message as Record<string, string>);
	const names = texts.flatMap((text) => [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]));
	return [...new Set(names)].sort().join(',');
}

describe('landing catalogs', () => {
	for (const locale of locales) {
		it(`${locale} has every English key with the same placeholders`, () => {
			const problems: string[] = [];
			for (const [key, english] of Object.entries(catalogs.en)) {
				const message = (catalogs[locale] as Record<string, unknown>)[key];
				if (!message) problems.push(`${key}: empty`);
				else if (placeholders(message) !== placeholders(english)) problems.push(`${key}: placeholders differ`);
			}
			expect(problems).toEqual([]);
		});
	}

	it('keeps engine and protocol names and false claims out of the pitch', () => {
		const problems = Object.values(catalogs).flatMap((catalog) =>
			pitch.flatMap((key) => {
				const text = String((catalog as Record<string, unknown>)[key]);
				return banned.filter((pattern) => pattern.test(text)).map((pattern) => `${key}: ${pattern}`);
			})
		);
		expect(problems).toEqual([]);
	});

	it('writes no em dash anywhere', () => {
		const dashed = Object.values(catalogs).flatMap((catalog) =>
			Object.entries(catalog as Record<string, unknown>).filter(([, text]) => JSON.stringify(text).includes('\u2014'))
		);
		expect(dashed).toEqual([]);
	});
});
