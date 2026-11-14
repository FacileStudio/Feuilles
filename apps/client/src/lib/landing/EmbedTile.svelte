<script lang="ts">
	import { Icon } from '@facile/muse';
	import { Bar, Stage, Tile, type Act } from '@facile/mise-en-scene';
	import { formatNumber, t } from '$lib/i18n/index.svelte';
	import { icons } from '$lib/icons';
	import MiniSheet from './MiniSheet.svelte';

	let shown = $state(false);
	let live = $state(false);

	const rows = $derived([
		{ label: t('demo.row.sales'), value: formatNumber(live ? 1240 : 1180), tone: live ? ('live' as const) : undefined },
		{ label: t('demo.row.costs'), value: formatNumber(760) }
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

<Tile title={t('demo.embed.title')} body={t('demo.embed.body')}>
	<Stage width={320} height={240} {script} {reset} {poster}>
		{#snippet children()}
			<div class="panel absolute top-5 left-5 flex h-[192px] w-[280px] flex-col gap-2.5 rounded-fc-lg bg-fc-page p-4">
				<span class="font-fc-title text-fc-sm font-bold text-fc-fg">{t('demo.embed.page')}</span>
				<Bar w="88%" />
				<Bar w="64%" />
				<div class="relative h-[96px]">
					<div class="lines flex flex-col gap-2.5 pt-1" class:lines-hidden={shown}>
						<Bar w="76%" />
						<Bar w="92%" />
						<Bar w="58%" />
					</div>
					<div class="embed absolute inset-x-0 top-0 flex flex-col gap-1.5 rounded-fc-md bg-fc-component p-2" class:embed-shown={shown}>
						<span class="flex items-center gap-1.5 text-[10px] leading-3 font-medium text-fc-fg-muted">
							<Icon icon={icons.logo} size={12} />Feuilles · {t('demo.book')}
						</span>
						<MiniSheet {rows} />
					</div>
				</div>
			</div>
		{/snippet}
	</Stage>
</Tile>

<style>
	.panel {
		box-shadow: 0 14px 28px -14px rgb(0 0 0 / 0.28);
	}
	.embed {
		opacity: 0;
		transform: translateY(8px) scale(0.97);
		transition:
			opacity 0.36s ease,
			transform 0.36s cubic-bezier(0.25, 0.8, 0.25, 1);
	}
	.lines {
		transition: opacity 0.3s ease;
	}
	.lines-hidden {
		opacity: 0;
	}
	.embed-shown {
		opacity: 1;
		transform: none;
	}
</style>
