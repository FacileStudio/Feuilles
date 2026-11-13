import { afterAll, beforeAll, expect, test } from 'bun:test';
import { chromium, type Browser, type Page } from 'playwright-core';
import { startStack, type Stack } from './stack';

let stack: Stack;
let browser: Browser;

beforeAll(async () => {
	stack = await startStack();
	browser = await chromium.launch({ headless: true });
}, 60_000);

afterAll(async () => {
	await browser?.close();
	await stack?.stop();
});

type Ink = { background: number; contrast: number };

/** The luminance of A1's first pixel and the contrast between its darkest and lightest pixels. */
function inkOfA1(page: Page): Promise<Ink> {
	return page.evaluate((): Ink => {
		const canvas = document.querySelector<HTMLCanvasElement>('.ic-worksheet-sheet-canvas');
		const context = canvas?.getContext('2d');
		if (!context) throw new Error('no sheet canvas');
		const ratio = window.devicePixelRatio || 1;
		const { data } = context.getImageData(34 * ratio, 31 * ratio, 70 * ratio, 18 * ratio);
		const channel = (c: number) => (c <= 10.31 ? c / 3295.4 : ((c / 255 + 0.055) / 1.055) ** 2.4);
		const levels: number[] = [];
		for (let i = 0; i < data.length; i += 4) levels.push(0.2126 * channel(data[i]) + 0.7152 * channel(data[i + 1]) + 0.0722 * channel(data[i + 2]));
		const [low, high] = [Math.min(...levels), Math.max(...levels)];
		return { background: levels[0], contrast: (high + 0.05) / (low + 0.05) };
	});
}

/** Waits until A1 is painted on a dark or light background with text at 4.5:1 or more. */
async function settles(page: Page, dark: boolean): Promise<void> {
	const start = performance.now();
	let ink = await inkOfA1(page);
	while ((ink.background < 0.5) !== dark || ink.contrast < 4.5) {
		if (performance.now() - start > 5_000) throw new Error(`A1 ink ${JSON.stringify(ink)} in the ${dark ? 'dark' : 'light'} theme`);
		await Bun.sleep(50);
		ink = await inkOfA1(page);
	}
}

test('cell text stays readable when the theme switches to dark and back', async () => {
	const answer = await fetch(`${stack.base}/api/workbooks`, {
		method: 'POST',
		headers: { cookie: `session=${stack.cookies.alice}`, 'content-type': 'application/json', 'x-facile-csrf': '1' },
		body: JSON.stringify({ title: 'Night shift', space_id: stack.space })
	});
	expect(answer.status).toBe(201);
	const { id } = (await answer.json()) as { id: string };
	const context = await browser.newContext({ colorScheme: 'light' });
	await context.addCookies([{ name: 'session', value: stack.cookies.alice, url: stack.base }]);
	const page = await context.newPage();
	await page.goto(`${stack.base}/w/${id}`);
	await page.getByRole('grid', { name: 'Sheet' }).waitFor();
	await page.waitForFunction(() => !document.querySelector('header [role="status"]'));
	await page.getByRole('grid', { name: 'Sheet' }).focus();
	await page.keyboard.type('Readable');
	await page.keyboard.press('Enter');
	await settles(page, false);
	await page.evaluate(() => document.documentElement.classList.add('dark'));
	await settles(page, true);
	await page.evaluate(() => document.documentElement.classList.replace('dark', 'light'));
	await settles(page, false);
}, 60_000);
