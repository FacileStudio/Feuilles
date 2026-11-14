<script lang="ts">
	import { tick } from 'svelte';
	import { Badge, Button, Icon } from '@facile/muse';
	import { Cursor, Stage, Tile, cursorAt, grow, type Act } from '@facile/mise-en-scene';
	import { icons } from '$lib/icons';
	import MiniSheet from './MiniSheet.svelte';

	const KEEP = [
		{ label: 'Rent', value: '900' },
		{ label: 'Ads', value: '300' }
	];
	const DRAFTS = [
		{ label: 'Draft', value: '0' },
		{ label: 'Draft 2', value: '0' }
	];

	let asked = $state(false);
	let done = $state(false);
	let cursor = $state(cursorAt());
	let approve = $state<HTMLElement>();

	const rows = $derived([...KEEP, ...(done ? [] : DRAFTS.map((row) => ({ ...row, tone: asked ? ('warning' as const) : undefined })))]);

	function reset() {
		asked = false;
		done = false;
		cursor.visible = false;
	}

	function poster() {
		asked = true;
	}

	async function script(act: Act) {
		await act.wait(700);
		asked = true;
		await tick();
		await act.wait(900);
		cursor.visible = true;
		await act.move(cursor, act.point(approve, 0.82, 0.8));
		await act.click(cursor);
		asked = false;
		done = true;
		await act.wait(400);
		cursor.visible = false;
		await act.wait(2200);
		reset();
		await act.wait(500);
	}
</script>

<Tile title="Agents ask first" body="Agents edit over MCP. Removing cells waits for your yes.">
	<Stage width={320} height={240} {script} {reset} {poster}>
		{#snippet children()}
			<div class="panel absolute top-5 left-5 h-[134px] w-[280px] rounded-fc-lg bg-fc-page p-3">
				<MiniSheet {rows} />
			</div>
			{#if asked}
				<section
					class="card absolute top-[146px] right-5 flex w-[236px] flex-col gap-2 rounded-fc-md bg-fc-surface p-2.5"
					transition:grow={{ duration: 300 }}
				>
					<div class="flex items-center gap-2">
						<Icon icon={icons.shield} size={16} class="shrink-0 text-fc-warning" />
						<Badge tone="warning">delete_rows</Badge>
						<span class="truncate text-fc-xs text-fc-fg">Rows 3 to 4</span>
					</div>
					<div class="flex items-center gap-2">
						<span bind:this={approve} class="inline-flex"><Button size="sm" icon={icons.check}>Approve</Button></span>
						<Button size="sm" variant="secondary" icon={icons.close}>Decline</Button>
					</div>
				</section>
			{/if}
			{#if done}
				<div
					class="card absolute top-[166px] right-5 flex items-center gap-2 rounded-fc-md bg-fc-surface px-3 py-2 text-fc-xs text-fc-fg"
					transition:grow={{ duration: 300 }}
				>
					<Icon icon={icons.check} size={16} class="text-fc-success" />
					<span class="font-medium">Done</span>
				</div>
			{/if}
			<Cursor {cursor} />
		{/snippet}
	</Stage>
</Tile>

<style>
	.panel,
	.card {
		box-shadow: 0 14px 28px -14px rgb(0 0 0 / 0.28);
	}
</style>
