<script lang="ts">
	import { Bar, Roll } from '@facile/mise-en-scene';

	interface Props {
		compact: boolean;
		tools: string;
		design: string;
		active: 'B5' | 'B6' | null;
		formula: string;
		total: string;
		peer: boolean;
		toolsCell?: HTMLElement;
		totalCell?: HTMLElement;
	}

	let { compact, tools, design, active, formula, total, peer, toolsCell = $bindable(), totalCell = $bindable() }: Props = $props();

	const ROWS = [
		{ label: 'Hosting', cost: '120' },
		{ label: 'Design', cost: '480' },
		{ label: 'Ads', cost: '300' },
		{ label: 'Tools', cost: '' }
	];

	const editing = $derived(active === 'B6' && total === '');
	const range = $derived(editing && formula.length >= 6);

	function cost(index: number) {
		if (index === 1) return design;
		if (index === 3) return tools;
		return ROWS[index].cost;
	}
</script>

<div class="sd-grid" class:sd-compact-grid={compact}>
	<span class="sd-corner"></span>
	<span class="sd-col">A</span>
	<span class="sd-col">B</span>
	<span class="sd-col">C</span>
	{#if !compact}<span class="sd-col">D</span>{/if}

	<span class="sd-row">1</span>
	<span class="sd-cell sd-strong">Item</span>
	<span class="sd-cell sd-strong sd-num">Cost</span>
	<span class="sd-cell"><Bar w="48%" /></span>
	{#if !compact}<span class="sd-cell"></span>{/if}

	{#each ROWS as row, index (row.label)}
		{@const address = `B${index + 2}`}
		<span class="sd-row">{index + 2}</span>
		<span class="sd-cell">{row.label}</span>
		{#if index === 3}
			<span bind:this={toolsCell} class="sd-cell sd-num" class:sd-range={range} class:sd-active={active === address}>{tools}</span>
		{:else}
			<span class="sd-cell sd-num" class:sd-range={range} class:sd-peer-cell={peer && index === 1}>
				{#if index === 1}<Roll value={cost(index)} />{:else}{cost(index)}{/if}
				{#if peer && index === 1}<span class="sd-tag"><span class="sd-dot"></span>Léa</span>{/if}
			</span>
		{/if}
		<span class="sd-cell"><Bar w={`${[62, 40, 55, 34][index]}%`} /></span>
		{#if !compact}<span class="sd-cell"></span>{/if}
	{/each}

	<span class="sd-row">6</span>
	<span class="sd-cell sd-strong">Total</span>
	<span bind:this={totalCell} class="sd-cell sd-num sd-strong" class:sd-active={active === 'B6'}>
		{#if editing}<span class="sd-typed">{formula}</span>{:else}<Roll value={total} />{/if}
	</span>
	<span class="sd-cell"></span>
	{#if !compact}<span class="sd-cell"></span>{/if}

	<span class="sd-row">7</span>
	<span class="sd-cell"></span>
	<span class="sd-cell"></span>
	<span class="sd-cell"></span>
	{#if !compact}<span class="sd-cell"></span>{/if}
</div>

<style>
	.sd-grid {
		display: grid;
		flex: 1;
		grid-template-columns: 44px 200px 150px 220px 1fr;
		grid-auto-rows: 32px;
		grid-template-rows: 24px;
		align-content: start;
		gap: 1px;
		min-height: 0;
		overflow: hidden;
		background: var(--mes-w-side);
		font-size: 13px;
		color: var(--color-fc-fg);
	}
	.sd-compact-grid {
		grid-template-columns: 30px 118px 96px 1fr;
		grid-auto-rows: 30px;
	}
	.sd-corner,
	.sd-col,
	.sd-row {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--mes-w-side);
		color: var(--color-fc-fg-muted);
		font-size: 11px;
		font-weight: 500;
	}
	.sd-cell {
		position: relative;
		display: flex;
		align-items: center;
		min-width: 0;
		padding: 0 10px;
		background: var(--mes-w-body);
		white-space: nowrap;
		transition: background 0.2s ease;
	}
	.sd-num {
		justify-content: flex-end;
		font-variant-numeric: tabular-nums;
	}
	.sd-strong {
		font-weight: 700;
	}
	.sd-range {
		background: color-mix(in oklch, var(--color-fc-accent) 10%, var(--mes-w-body));
	}
	.sd-active {
		box-shadow: inset 0 0 0 2px var(--color-fc-accent);
	}
	.sd-peer-cell {
		box-shadow: inset 0 0 0 2px var(--color-fc-chart-1);
	}
	.sd-typed {
		width: 100%;
		font-family: var(--font-fc-mono);
		font-size: 12px;
		font-weight: 400;
		text-align: left;
	}
	.sd-tag {
		position: absolute;
		top: -18px;
		left: 0;
		z-index: 1;
		display: flex;
		align-items: center;
		gap: 4px;
		height: 18px;
		padding: 0 6px;
		border-radius: 4px;
		background: var(--color-fc-component);
		font-size: 11px;
		font-weight: 500;
		color: var(--color-fc-fg);
	}
	.sd-dot {
		width: 8px;
		height: 8px;
		border-radius: 999px;
		background: var(--color-fc-chart-1);
	}
</style>
