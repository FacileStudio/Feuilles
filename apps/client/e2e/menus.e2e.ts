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

/** Creates a workbook, in the shared space unless `personal`, and opens a signed-in page on `path`. */
async function signedIn(title: string, path: (id: string) => string, personal = false): Promise<Page> {
	const answer = await fetch(`${stack.base}/api/workbooks`, {
		method: 'POST',
		headers: { cookie: `session=${stack.cookies.alice}`, 'content-type': 'application/json', 'x-facile-csrf': '1' },
		body: JSON.stringify({ title, space_id: personal ? undefined : stack.space })
	});
	expect(answer.status).toBe(201);
	const { id } = (await answer.json()) as { id: string };
	const context = await browser.newContext();
	await context.addCookies([{ name: 'session', value: stack.cookies.alice, url: stack.base }]);
	const page = await context.newPage();
	await page.goto(`${stack.base}${path(id)}`);
	return page;
}

/** The label of the focused menu item. */
const focused = (page: Page) => page.evaluate(() => document.activeElement?.closest('[role="menuitem"]')?.textContent?.trim() ?? null);

/** Walks an open menu with the keyboard and checks every item carries an icon. */
async function walksMenu(page: Page, first: string, last: string): Promise<void> {
	const menu = page.getByRole('menu');
	await menu.waitFor();
	const items = menu.getByRole('menuitem');
	const count = await items.count();
	for (let i = 0; i < count; i++) expect(await items.nth(i).locator('svg').count()).toBe(1);
	expect(await focused(page)).toBe(first);
	await page.keyboard.press('End');
	expect(await focused(page)).toBe(last);
	await page.keyboard.press('Home');
	expect(await focused(page)).toBe(first);
	await page.keyboard.press('ArrowUp');
	expect(await focused(page)).toBe(last);
	await page.keyboard.press('ArrowDown');
	expect(await focused(page)).toBe(first);
	await page.keyboard.press('Escape');
	await menu.waitFor({ state: 'detached' });
}

test('toolbar icons show muse tooltips, at once along the toolbar and on keyboard focus', async () => {
	const page = await signedIn('Tooltips', (id) => `/w/${id}`);
	const toolbar = page.getByRole('toolbar', { name: 'Formatting' });
	await toolbar.waitFor();
	await page.waitForFunction(() => !document.querySelector('header [role="status"]'));
	expect(await toolbar.locator('[title]').count()).toBe(0);
	await toolbar.getByRole('button', { name: 'Bold' }).hover();
	await page.locator('[role="tooltip"]').filter({ hasText: 'Bold' }).waitFor({ timeout: 2_000 });
	await toolbar.getByRole('button', { name: 'Italic' }).hover();
	await page.locator('[role="tooltip"]').filter({ hasText: 'Italic' }).waitFor({ timeout: 250 });
	await page.mouse.move(0, 400);
	await page.waitForFunction(() => !document.querySelector('[role="tooltip"]'));
	await page.keyboard.press('Shift');
	await toolbar.getByRole('button', { name: 'Underline' }).focus();
	await page.locator('[role="tooltip"]').filter({ hasText: 'Underline' }).waitFor({ timeout: 250 });
}, 60_000);

test('the grid menu walks with the keyboard and hands focus back to the grid', async () => {
	const page = await signedIn('Grid menu', (id) => `/w/${id}`);
	const grid = page.getByRole('grid', { name: 'Sheet' });
	await grid.waitFor();
	await page.waitForFunction(() => !document.querySelector('header [role="status"]'));
	await grid.click({ button: 'right', position: { x: 120, y: 60 } });
	await walksMenu(page, 'Copy', 'Clear contents');
	expect(await page.evaluate(() => document.activeElement?.getAttribute('aria-label'))).toBe('Sheet');
}, 60_000);

test('the workbook card menu walks with the keyboard and hands focus back to the card', async () => {
	const page = await signedIn('Card menu', () => '/', true);
	const card = page.getByRole('link', { name: /Card menu/ });
	await card.waitFor();
	await card.click({ button: 'right' });
	await walksMenu(page, 'Open', 'Move to trash');
	expect(await page.evaluate(() => document.activeElement?.textContent)).toContain('Card menu');
}, 60_000);
