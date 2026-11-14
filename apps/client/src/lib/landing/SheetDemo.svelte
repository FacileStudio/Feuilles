<script lang="ts">
	import { Bar, Cursor, Stage, Window, cursorAt, type Act } from '@facile/mise-en-scene';
	import { Icon } from '@facile/muse';
	import { icons } from '$lib/icons';
	import SheetGrid from './SheetGrid.svelte';

	const FORMULA = '=SUM(B2:B5)';
	const TOOLS = [icons.undo, icons.redo, null, icons.bold, icons.italic, icons.underline, icons.fill, null, icons.alignLeft, icons.alignCenter, icons.alignRight, null, icons.currency, icons.percent];

	let tools = $state('');
	let design = $state('480');
	let active = $state<'B5' | 'B6' | null>(null);
	let formula = $state('');
	let total = $state('');
	let peer = $state(false);
	let cursor = $state(cursorAt());
	let toolsCell = $state<HTMLElement>();
	let totalCell = $state<HTMLElement>();

	const bar = $derived(active === 'B5' ? tools : active === 'B6' ? formula : '');

	function reset() {
		tools = '';
		design = '480';
		active = null;
		formula = '';
		total = '';
		peer = false;
		cursor.visible = false;
	}

	function poster() {
		tools = '90';
		design = '520';
		active = 'B6';
		formula = FORMULA;
		total = '1,030';
		peer = true;
	}

	async function script(act: Act) {
		await act.wait(700);
		cursor.visible = true;
		await act.move(cursor, act.point(toolsCell, 0.3, 0.7));
		await act.click(cursor, () => (active = 'B5'));
		await act.wait(250);
		await act.type('90', (value) => (tools = value), 160);
		await act.wait(500);
		await act.move(cursor, act.point(totalCell, 0.85, 0.75));
		await act.click(cursor, () => (active = 'B6'));
		await act.wait(300);
		await act.type(FORMULA, (value) => (formula = value), 75);
		await act.wait(700);
		total = '990';
		await act.wait(1200);
		cursor.visible = false;
		peer = true;
		await act.wait(900);
		design = '520';
		total = '1,030';
		await act.wait(2400);
		reset();
		await act.wait(600);
	}
</script>

{#snippet header(compact: boolean)}
	<div class="sd-head">
		<span class="sd-back"><Icon icon={icons.chevronLeft} size={16} />{#if !compact}Workbooks{/if}</span>
		<span class="sd-title">Q4 budget</span>
		<span class="sd-people">
			<span class="sd-avatar sd-you">Y</span>
			{#if peer}<span class="sd-avatar sd-peer">L</span>{/if}
		</span>
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
	label="A Feuilles workbook: a cost is typed in, a SUM formula totals the column, and the total updates when a teammate edits a cell."
>
	{#snippet children({ compact })}
		{#if compact}
			<Window style="top:8px;left:0;width:358px;height:388px">
				{@render header(true)}
				{@render formulaBar()}
				<SheetGrid compact {tools} {design} {active} {formula} {total} {peer} bind:toolsCell bind:totalCell />
				{@render tabs()}
			</Window>
		{:else}
			<Window style="top:8px;left:0;width:960px;height:440px">
				{@render header(false)}
				{@render toolbar()}
				{@render formulaBar()}
				<SheetGrid compact={false} {tools} {design} {active} {formula} {total} {peer} bind:toolsCell bind:totalCell />
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
	.sd-people {
		display: flex;
		gap: 4px;
	}
	.sd-avatar {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 700;
	}
	.sd-you {
		background: var(--color-fc-accent);
		color: var(--color-fc-accent-fg);
	}
	.sd-peer {
		background: var(--color-fc-chart-1);
		color: var(--color-fc-fg);
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
