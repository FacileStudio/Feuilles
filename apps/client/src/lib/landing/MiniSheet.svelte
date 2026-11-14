<script lang="ts">
	import { Roll, grow } from '@facile/mise-en-scene';

	interface Row {
		label: string;
		value: string;
		tone?: 'warning' | 'live';
	}

	let { rows }: { rows: Row[] } = $props();
</script>

<div class="ms">
	<div class="ms-row ms-head"><span class="ms-n"></span><span class="ms-cell">A</span><span class="ms-cell">B</span></div>
	{#each rows as row, index (row.label)}
		<div class="ms-wrap" out:grow={{ duration: 320 }}>
			<div class="ms-row" class:ms-warning={row.tone === 'warning'} class:ms-live={row.tone === 'live'}>
				<span class="ms-n">{index + 1}</span>
				<span class="ms-cell ms-body">{row.label}</span>
				<span class="ms-cell ms-body ms-num"><Roll value={row.value} /></span>
			</div>
		</div>
	{/each}
</div>

<style>
	.ms {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: var(--radius-fc-sm);
		background: var(--color-fc-component);
		font-size: 11px;
		color: var(--color-fc-fg);
	}
	.ms-wrap {
		padding-top: 1px;
	}
	.ms-row {
		display: grid;
		grid-template-columns: 22px 1fr 1fr;
		gap: 1px;
		height: 22px;
	}
	.ms-head {
		height: 16px;
		color: var(--color-fc-fg-muted);
		font-size: 10px;
		font-weight: 500;
	}
	.ms-n {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-fc-fg-muted);
		font-size: 10px;
	}
	.ms-head .ms-cell {
		justify-content: center;
		padding: 0;
	}
	.ms-cell {
		display: flex;
		align-items: center;
		min-width: 0;
		padding: 0 6px;
		white-space: nowrap;
	}
	.ms-body {
		background: var(--color-fc-page);
		transition:
			background 0.25s ease,
			box-shadow 0.25s ease;
	}
	.ms-num {
		justify-content: flex-end;
		font-variant-numeric: tabular-nums;
	}
	.ms-warning .ms-body {
		background: color-mix(in oklch, var(--color-fc-warning) 16%, var(--color-fc-page));
	}
	.ms-live .ms-num {
		box-shadow: inset 0 0 0 1.5px var(--color-fc-accent);
	}
</style>
