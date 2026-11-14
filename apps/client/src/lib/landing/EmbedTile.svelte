<script lang="ts">
	import { Icon } from '@facile/muse';
	import { Bar, Stage, Tile, grow, type Act } from '@facile/mise-en-scene';
	import { icons } from '$lib/icons';
	import MiniSheet from './MiniSheet.svelte';

	let shown = $state(false);
	let live = $state(false);

	const rows = $derived([
		{ label: 'Sales', value: live ? '1,240' : '1,180', tone: live ? ('live' as const) : undefined },
		{ label: 'Costs', value: '760' }
	]);

	function reset() {
		shown = false;
		live = false;
	}

	function poster() {
		shown = true;
		live = true;
	}

	async function script(act: Act) {
		await act.wait(700);
		shown = true;
		await act.wait(1600);
		live = true;
		await act.wait(2600);
		reset();
		await act.wait(600);
	}
</script>

<Tile title="Inside your docs" body="Embed a workbook in Facile Docs. It stays live.">
	<Stage width={320} height={240} {script} {reset} {poster}>
		{#snippet children()}
			<div class="panel absolute top-5 left-5 flex h-[192px] w-[280px] flex-col gap-2.5 rounded-fc-lg bg-fc-page p-4">
				<span class="font-fc-title text-fc-sm font-bold text-fc-fg">Q4 review</span>
				<Bar w="88%" />
				<Bar w="64%" />
				{#if shown}
					<div class="flex flex-col gap-1.5 rounded-fc-md bg-fc-component p-2" transition:grow={{ duration: 360 }}>
						<span class="flex items-center gap-1.5 text-[10px] leading-3 font-medium text-fc-fg-muted">
							<Icon icon={icons.logo} size={12} />Feuilles · Q4 budget
						</span>
						<MiniSheet {rows} />
					</div>
				{:else}
					<Bar w="76%" />
				{/if}
			</div>
		{/snippet}
	</Stage>
</Tile>

<style>
	.panel {
		box-shadow: 0 14px 28px -14px rgb(0 0 0 / 0.28);
	}
</style>
