<script module lang="ts">
	export interface Peer {
		name: string;
		color: string;
		from: string | null;
		to?: string;
	}
</script>

<script lang="ts">
	import { Bar, Roll } from '@facile/mise-en-scene';
	import { t } from '$lib/i18n/index.svelte';

	interface Props {
		compact: boolean;
		values: Record<string, string>;
		formatted: boolean;
		active: string | null;
		formula: string;
		total: string;
		peers: Peer[];
		cells?: Record<string, HTMLElement>;
	}

	let { compact, values, formatted, active, formula, total, peers, cells = $bindable({}) }: Props = $props();

	const labels = $derived([t('demo.row.hosting'), t('demo.row.design'), t('demo.row.ads'), t('demo.row.tools')]);
	const head = $derived([t('demo.head.item'), t('demo.head.cost'), t('demo.head.notes')]);
	const BARS = [62, 40, 55, 34];

	const editing = $derived(active === 'B6' && total === '');
	const range = $derived(editing && formula.length >= 6);
	const columns = $derived(compact ? ['A', 'B', 'C'] : ['A', 'B', 'C', 'D']);

	const boxes = $derived(
		peers.flatMap((peer) => {
			const start = peer.from ? cells[peer.from] : undefined;
			const end = cells[peer.to ?? peer.from ?? ''] ?? start;
			if (!start || !end) return [];
			const left = start.offsetLeft;
			const top = start.offsetTop;
			return [{ ...peer, left, top, width: end.offsetLeft + end.offsetWidth - left, height: end.offsetTop + end.offsetHeight - top }];
		})
	);
</script>

<div class="sd-grid" class:sd-compact-grid={compact}>
	<span class="sd-corner"></span>
	{#each columns as column (column)}<span class="sd-col">{column}</span>{/each}

	<span class="sd-row">1</span>
	{#each head as label, index (index)}
		<span bind:this={cells[`${columns[index]}1`]} class="sd-cell" class:sd-num={index === 1} class:sd-head={formatted}>{label}</span>
	{/each}
	{#if !compact}<span class="sd-cell"></span>{/if}

	{#each labels as label, index (index)}
		{@const address = `B${index + 2}`}
		<span class="sd-row">{index + 2}</span>
		<span class="sd-cell">{label}</span>
		<span bind:this={cells[address]} class="sd-cell sd-num" class:sd-range={range}>{values[address] ?? ''}</span>
		<span class="sd-cell"><Bar w={`${BARS[index]}%`} /></span>
		{#if !compact}<span class="sd-cell"></span>{/if}
	{/each}

	<span class="sd-row">6</span>
	<span class="sd-cell sd-strong">{t('demo.total')}</span>
	<span bind:this={cells.B6} class="sd-cell sd-num sd-strong" class:sd-active={active === 'B6'}>
		{#if editing}<span class="sd-typed">{formula}</span>{:else}<Roll value={total} />{/if}
	</span>
	<span class="sd-cell"></span>
	{#if !compact}<span class="sd-cell"></span>{/if}

	<span class="sd-row">7</span>
	{#each columns as column (column)}<span class="sd-cell"></span>{/each}

	{#each boxes as box (box.name)}
		<div
			class="sd-peer pointer-events-none absolute z-[5] rounded-[2px]"
			style:left="{box.left}px"
			style:top="{box.top}px"
			style:width="{box.width}px"
			style:height="{box.height}px"
			style:outline="2px solid {box.color}"
		>
			<span
				class="absolute -top-5 left-0 flex items-center gap-1 rounded-fc-sm bg-fc-component px-1.5 text-[11px] leading-5 font-medium whitespace-nowrap text-fc-fg"
			>
				<span class="size-2 rounded-full" style:background-color={box.color}></span>
				{box.name}
			</span>
		</div>
	{/each}
</div>

<style>
	.sd-grid {
		position: relative;
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
		transition:
			background 0.25s ease,
			font-weight 0.25s ease;
	}
	.sd-num {
		justify-content: flex-end;
		font-variant-numeric: tabular-nums;
	}
	.sd-strong,
	.sd-head {
		font-weight: 700;
	}
	.sd-head {
		background: color-mix(in oklch, var(--color-fc-fg) 7%, var(--mes-w-body));
	}
	.sd-range {
		background: color-mix(in oklch, var(--color-fc-accent) 10%, var(--mes-w-body));
	}
	.sd-active {
		box-shadow: inset 0 0 0 2px var(--color-fc-accent);
	}
	.sd-peer {
		outline-offset: -1px;
		transition:
			left 0.35s cubic-bezier(0.25, 0.8, 0.25, 1),
			top 0.35s cubic-bezier(0.25, 0.8, 0.25, 1),
			width 0.35s cubic-bezier(0.25, 0.8, 0.25, 1);
	}
	.sd-typed {
		width: 100%;
		font-family: var(--font-fc-mono);
		font-size: 12px;
		font-weight: 400;
		text-align: left;
	}
</style>
