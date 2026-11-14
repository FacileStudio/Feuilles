<script lang="ts">
	import { Icon } from '@facile/muse';
	import { Stage, Tile, type Act } from '@facile/mise-en-scene';
	import { scale } from 'svelte/transition';
	import { icons } from '$lib/icons';
	import WorkbookPreview from '$lib/workbooks/WorkbookPreview.svelte';

	const BOOK = { sheets: [], sheet_count: 3 };

	let file = $state<'away' | 'ready' | 'over' | 'gone'>('away');
	let imported = $state(false);

	function reset() {
		file = 'away';
		imported = false;
	}

	function poster() {
		file = 'gone';
		imported = true;
	}

	async function script(act: Act) {
		await act.wait(400);
		file = 'ready';
		await act.wait(700);
		file = 'over';
		await act.wait(1100);
		file = 'gone';
		await act.wait(300);
		imported = true;
		await act.wait(2800);
		reset();
		await act.wait(700);
	}
</script>

<Tile title="Bring your xlsx" body="Drop an xlsx or csv file. It becomes a workbook.">
	<Stage width={320} height={240} {script} {reset} {poster}>
		{#snippet children()}
			<div class="panel absolute top-5 left-5 flex h-[192px] w-[280px] gap-3 rounded-fc-lg bg-fc-page p-3">
				<div
					class="drop flex w-[104px] shrink-0 flex-col items-center justify-center gap-1.5 rounded-fc-md p-2 text-center"
					class:drop-over={file === 'over'}
				>
					<Icon icon={icons.upload} size={22} class="text-fc-fg-muted" />
					<span class="text-[11px] leading-4 font-medium text-fc-fg">Import an .xlsx</span>
				</div>
				<div class="flex min-w-0 flex-1 flex-col justify-center">
					{#if imported}
						<div class="flex flex-col gap-2 rounded-fc-md bg-fc-component p-2" in:scale={{ start: 0.92, duration: 320 }}>
							<WorkbookPreview workbook={BOOK} />
							<span class="flex flex-col gap-0.5 px-0.5">
								<span class="truncate text-fc-xs font-medium text-fc-fg">Budget 2026</span>
								<span class="text-[10px] leading-3 text-fc-fg-muted">3 sheets · Just now</span>
							</span>
						</div>
					{:else}
						<div class="flex flex-1 flex-col items-center justify-center gap-2 rounded-fc-md bg-fc-surface/50">
							<span class="flex size-8 items-center justify-center rounded-full bg-fc-component text-fc-fg-muted">
								<Icon icon={icons.plus} size={16} />
							</span>
						</div>
					{/if}
				</div>
			</div>
			<div class="chip absolute flex items-center gap-1.5 rounded-fc-sm bg-fc-component px-2 py-1.5 text-[11px] font-medium text-fc-fg" data-at={file}>
				<Icon icon={icons.workbook} size={14} class="text-fc-success" />
				budget-2026.xlsx
			</div>
		{/snippet}
	</Stage>
</Tile>

<style>
	.panel,
	.chip {
		box-shadow: 0 14px 28px -14px rgb(0 0 0 / 0.28);
	}
	.drop {
		background: var(--color-fc-surface);
		transition: background 0.25s ease;
	}
	.drop-over {
		background: color-mix(in oklch, var(--color-fc-accent) 12%, var(--color-fc-surface));
	}
	.chip {
		top: 8px;
		left: 176px;
		opacity: 0;
		transition:
			top 0.9s cubic-bezier(0.25, 0.8, 0.25, 1),
			left 0.9s cubic-bezier(0.25, 0.8, 0.25, 1),
			opacity 0.25s ease;
	}
	.chip[data-at='ready'] {
		opacity: 1;
	}
	.chip[data-at='over'] {
		opacity: 1;
		top: 112px;
		left: 30px;
	}
	.chip[data-at='gone'] {
		top: 112px;
		left: 30px;
		opacity: 0;
	}
</style>
