<script lang="ts">
	import { Bar, Cursor, Stage, Window, cursorAt, type Act } from '@facile/mise-en-scene';
	import { Icon, Presence, USER_COLORS } from '@facile/muse';
	import { icons } from '$lib/icons';
	import SheetGrid, { type Peer } from './SheetGrid.svelte';

	const FORMULA = '=SUM(B2:B5)';
	const COSTS = ['120', '480', '300', '90'];
	const LEA = USER_COLORS[1];
	const MALIK = USER_COLORS[3];
	const PEERS = [
		{ id: 'lea', name: 'Léa', color: LEA },
		{ id: 'malik', name: 'Malik', color: MALIK }
	];
	const TOOLS = [icons.undo, icons.redo, null, icons.bold, icons.italic, icons.underline, icons.fill, null, icons.alignLeft, icons.alignCenter, icons.alignRight, null, icons.currency, icons.percent];

	let values = $state<Record<string, string>>({});
	let landed = $state<string[]>([]);
	let formatted = $state(false);
	let active = $state<string | null>(null);
	let formula = $state('');
	let summed = $state(false);
	let lea = $state<string | null>(null);
	let malik = $state<{ from: string | null; to: string }>({ from: null, to: 'A1' });
	let cursor = $state(cursorAt());
	let cells = $state<Record<string, HTMLElement>>({});

	const total = $derived(summed ? landed.reduce((sum, address) => sum + Number(values[address]), 0).toLocaleString('en-US') : '');
	const bar = $derived(active === 'B6' ? formula : '');
	const peers = $derived<Peer[]>([
		{ name: 'Léa', color: LEA, from: lea },
		{ name: 'Malik', color: MALIK, ...malik }
	]);

	function reset() {
		values = {};
		landed = [];
		formatted = false;
		active = null;
		formula = '';
		summed = false;
		lea = null;
		malik = { from: null, to: 'A1' };
		cursor.visible = false;
	}

	function poster() {
		values = { B2: '120', B3: '480', B4: '3' };
		landed = ['B2', 'B3'];
		formatted = true;
		active = 'B6';
		formula = '=SUM(B2:B';
		lea = 'B4';
		malik = { from: 'A1', to: 'C1' };
	}

	async function fillCosts(act: Act) {
		await act.wait(1300);
		for (const [index, cost] of COSTS.entries()) {
			const address = `B${index + 2}`;
			lea = address;
			await act.wait(420);
			await act.type(cost, (value) => (values[address] = value), 130);
			landed.push(address);
			await act.wait(480);
		}
	}

	async function writeSum(act: Act) {
		await act.wait(400);
		cursor.visible = true;
		await act.move(cursor, act.point(cells.B6, 0.85, 0.75));
		await act.click(cursor, () => (active = 'B6'));
		await act.wait(250);
		await act.type(FORMULA, (value) => (formula = value), 80);
		await act.wait(300);
		summed = true;
		cursor.visible = false;
	}

	async function formatHeader(act: Act) {
		await act.wait(2400);
		malik = { from: 'A1', to: 'A1' };
		await act.wait(500);
		malik = { from: 'A1', to: 'B1' };
		await act.wait(260);
		malik = { from: 'A1', to: 'C1' };
		await act.wait(700);
		formatted = true;
	}

	async function script(act: Act) {
		await Promise.all([fillCosts(act), writeSum(act), formatHeader(act)]);
		await act.wait(2400);
		reset();
		await act.wait(600);
	}
</script>

{#snippet header(compact: boolean)}
	<div class="sd-head">
		<span class="sd-back"><Icon icon={icons.chevronLeft} size={16} />{#if !compact}Workbooks{/if}</span>
		<span class="sd-title">Q4 budget</span>
		<Presence peers={PEERS} />
		<span class="sd-icon"><Icon icon={icons.download} size={16} /></span>
	</div>
{/snippet}

{#snippet toolbar()}
	<div class="sd-toolbar">
		{#each TOOLS as tool, index (index)}
			{#if tool}<span class="sd-icon"><Icon icon={tool} size={16} /></span>{:else}<span class="sd-gap"></span>{/if}
		{/each}
	</div>
{/snippet}

{#snippet formulaBar()}
	<div class="sd-formula">
		<span class="sd-address">{active ?? 'A1'}</span>
		<span class="sd-input">{bar}</span>
	</div>
{/snippet}

{#snippet tabs()}
	<div class="sd-tabs">
		<span class="sd-icon"><Icon icon={icons.plus} size={16} /></span>
		<span class="sd-tab">Budget</span>
		<span class="sd-tab-idle"><Bar w="44px" /></span>
	</div>
{/snippet}

<Stage
	width={960}
	height={456}
	compact={{ below: 640, width: 358, height: 404 }}
	{script}
	{reset}
	{poster}
	class="w-full max-w-[960px]"
	label="A Feuilles workbook edited by three people at once: Léa fills in the costs, you write a SUM that updates as they land, and Malik formats the header row."
>
	{#snippet children({ compact })}
		{#if compact}
			<Window style="top:8px;left:0;width:358px;height:388px">
				{@render header(true)}
				{@render formulaBar()}
				<SheetGrid compact {values} {formatted} {active} {formula} {total} {peers} bind:cells />
				{@render tabs()}
			</Window>
		{:else}
			<Window style="top:8px;left:0;width:960px;height:440px">
				{@render header(false)}
				{@render toolbar()}
				{@render formulaBar()}
				<SheetGrid compact={false} {values} {formatted} {active} {formula} {total} {peers} bind:cells />
				{@render tabs()}
			</Window>
		{/if}
		<Cursor {cursor} />
	{/snippet}
</Stage>

<style>
	.sd-head {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 44px;
		padding: 0 12px;
		color: var(--color-fc-fg);
		font-size: 13px;
	}
	.sd-back {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--color-fc-fg-muted);
		font-size: 12px;
	}
	.sd-title {
		flex: 1;
		min-width: 0;
		font-family: var(--font-fc-title);
		font-size: 15px;
		font-weight: 700;
		white-space: nowrap;
	}
	.sd-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		color: var(--color-fc-fg-muted);
	}
	.sd-gap {
		width: 10px;
	}
	.sd-toolbar,
	.sd-formula,
	.sd-tabs {
		display: flex;
		flex: none;
		align-items: center;
		gap: 2px;
		padding: 0 8px;
		background: var(--mes-w-side);
	}
	.sd-toolbar {
		height: 36px;
	}
	.sd-formula {
		gap: 12px;
		height: 36px;
		padding: 0 12px;
		font-family: var(--font-fc-mono);
		font-size: 12px;
		color: var(--color-fc-fg);
	}
	.sd-toolbar + .sd-formula {
		padding-top: 2px;
	}
	.sd-address {
		width: 40px;
		color: var(--color-fc-fg-muted);
	}
	.sd-input {
		white-space: nowrap;
	}
	.sd-tabs {
		gap: 6px;
		height: 40px;
		margin-top: auto;
	}
	.sd-tab {
		padding: 5px 12px;
		border-radius: 6px;
		background: var(--mes-w-body);
		font-size: 12px;
		font-weight: 500;
		color: var(--color-fc-fg);
	}
	.sd-tab-idle {
		display: flex;
		padding: 0 12px;
	}
</style>
