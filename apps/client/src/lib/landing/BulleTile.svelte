<script lang="ts">
	import { Badge, Icon } from '@facile/muse';
	import { Stage, Stream, Tile, type Act } from '@facile/mise-en-scene';
	import { fly } from 'svelte/transition';
	import { formatNumber, t } from '$lib/i18n/index.svelte';
	import { icons } from '$lib/icons';
	import MiniSheet from './MiniSheet.svelte';

	let asked = $state(false);
	let words = $state(0);
	let filled = $state(false);

	const reply = $derived(t('demo.bulle.reply'));
	const rows = $derived([
		{ label: t('demo.row.rent'), value: formatNumber(900) },
		{ label: t('demo.row.ads'), value: formatNumber(300) },
		{ label: t('demo.row.tools'), value: formatNumber(120) },
		{ label: t('demo.total'), value: filled ? formatNumber(1320) : '', tone: filled ? ('live' as const) : undefined }
	]);

	function reset() {
		asked = false;
		words = 0;
		filled = false;
	}

	function poster() {
		asked = true;
		words = reply.split(' ').length;
		filled = true;
	}

	async function script(act: Act) {
		await act.wait(700);
		asked = true;
		await act.wait(1100);
		filled = true;
		await act.wait(300);
		await act.count(reply.split(' ').length, (value) => (words = value), 110);
		await act.wait(2600);
		reset();
		await act.wait(600);
	}
</script>

<Tile title={t('demo.bulle.title')} body={t('demo.bulle.body')}>
	{#snippet badge()}<Badge>Pro</Badge>{/snippet}
	<Stage width={320} height={240} {script} {reset} {poster}>
		{#snippet children()}
			<div class="panel absolute top-5 left-5 h-[132px] w-[280px] rounded-fc-lg bg-fc-page p-3">
				<MiniSheet {rows} />
			</div>
			{#if asked}
				<section
					class="card absolute top-[146px] right-5 flex w-[236px] flex-col gap-2 rounded-fc-lg bg-fc-surface p-2.5"
					transition:fly={{ y: 10, duration: 300 }}
				>
					<span class="self-end rounded-fc-lg bg-fc-component px-3 py-1 text-fc-xs text-fc-fg">{t('demo.bulle.ask')}</span>
					<div class="flex items-start gap-2">
						<Icon icon={icons.bulle} size={18} class="shrink-0 text-fc-fg" />
						<Stream text={reply} count={words} class="m-0 min-h-8 text-fc-xs leading-4 text-fc-fg" />
					</div>
				</section>
			{/if}
		{/snippet}
	</Stage>
</Tile>

<style>
	.panel,
	.card {
		box-shadow: 0 14px 28px -14px rgb(0 0 0 / 0.28);
	}
</style>
