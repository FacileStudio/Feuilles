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

async function pageAs(who: keyof Stack['cookies'], path: string): Promise<Page> {
	const context = await browser.newContext();
	await context.addCookies([{ name: 'session', value: stack.cookies[who], url: stack.base }]);
	const page = await context.newPage();
	await page.goto(`${stack.base}${path}`);
	return page;
}

async function createWorkbook(): Promise<string> {
	const answer = await fetch(`${stack.base}/api/workbooks`, {
		method: 'POST',
		headers: { cookie: `session=${stack.cookies.alice}`, 'content-type': 'application/json', 'x-facile-csrf': '1' },
		body: JSON.stringify({ title: 'Shared budget', space_id: stack.space })
	});
	expect(answer.status).toBe(201);
	return ((await answer.json()) as { id: string }).id;
}

async function live(page: Page): Promise<void> {
	await page.getByRole('grid', { name: 'Sheet' }).waitFor();
	await page.getByText('Live').waitFor();
	await page.getByRole('grid', { name: 'Sheet' }).focus();
}

const address = (page: Page) => page.getByLabel('Selected cell', { exact: true });

async function select(page: Page, row: number): Promise<void> {
	const current = Number((await address(page).textContent())?.slice(1) ?? 1);
	const key = row < current ? 'ArrowUp' : 'ArrowDown';
	for (let i = 0; i < Math.abs(row - current); i += 1) await page.keyboard.press(key);
	await expect(address(page).textContent()).resolves.toBe(`A${row}`);
}

async function enter(page: Page, row: number, input: string): Promise<void> {
	await select(page, row);
	await page.keyboard.type(input);
	await page.keyboard.press('Enter');
}

const value = (page: Page) => page.getByLabel('Value of the selected cell').textContent();

async function shows(page: Page, row: number, expected: string, within: number): Promise<number> {
	const start = performance.now();
	await select(page, row);
	while ((await value(page)) !== expected) {
		if (performance.now() - start > within) throw new Error(`A${row} shows ${await value(page)}, not ${expected}`);
		await Bun.sleep(20);
	}
	return performance.now() - start;
}

test('two members edit one workbook live, a reload keeps it, an outsider sees no access', async () => {
	const id = await createWorkbook();
	const [alice, bob] = await Promise.all([pageAs('alice', `/w/${id}`), pageAs('bob', `/w/${id}`)]);
	await Promise.all([live(alice), live(bob)]);
	for (const [row, input] of [[1, '1'], [2, '2'], [3, '3']] as const) await enter(alice, row, input);
	await shows(bob, 3, '3', 5_000);
	await Promise.all([select(alice, 4), select(bob, 2)]);
	await Promise.all([enter(alice, 4, '=SUM(A1:A3)'), enter(bob, 2, '10')]);
	const waited = await Promise.all([shows(alice, 4, '14', 1_000), shows(bob, 4, '14', 1_000)]);
	expect(Math.max(...waited)).toBeLessThan(1_000);
	console.log(`both pages showed 14 after ${Math.round(Math.max(...waited))} ms`);
	await alice.reload();
	await live(alice);
	await shows(alice, 4, '14', 5_000);
	await shows(alice, 2, '10', 1_000);
	const outsider = await pageAs('outsider', `/w/${id}`);
	await outsider.getByText('No access').waitFor({ timeout: 10_000 });
}, 120_000);

test('the API serves the icons and manifest of the built client', async () => {
	const want = {
		'/favicon.ico': 'image/vnd.microsoft.icon',
		'/favicon.svg': 'image/svg+xml',
		'/apple-touch-icon.png': 'image/png',
		'/icon-192.png': 'image/png',
		'/icon-512.png': 'image/png',
		'/site.webmanifest': 'application/manifest+json'
	};
	for (const [path, type] of Object.entries(want)) {
		const answer = await fetch(`${stack.base}${path}`);
		expect([path, answer.status, answer.headers.get('content-type')]).toEqual([path, 200, type]);
	}
	const page = await pageAs('alice', '/');
	const href = (await page.locator('link[rel="icon"]').getAttribute('href')) ?? '';
	expect(new URL(href, stack.base).pathname).toBe('/favicon.svg');
});
