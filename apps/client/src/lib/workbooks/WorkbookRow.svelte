<script lang="ts">
	import { Icon, Menu, type MenuEntry } from '@facile/muse';
	import type { Workbook } from '$lib/backend';
	import { icons } from '$lib/icons';
	import { ago } from './format';

	let { workbook, actions, href }: { workbook: Workbook; actions: MenuEntry[]; href?: string } = $props();

	const sheets = $derived(workbook.sheets.length === 1 ? '1 sheet' : `${workbook.sheets.length} sheets`);
	const when = $derived(workbook.trashed_at ? `Trashed ${ago(workbook.trashed_at)}` : `Edited ${ago(workbook.updated_at)}`);
</script>

<li class="flex min-h-14 items-center gap-3 rounded-fc-md bg-fc-component px-4 py-2 transition-colors hover:bg-fc-surface">
	<Icon icon={icons.workbook} size={20} class="text-fc-fg-muted" />
	{#if href}
		<a {href} class="min-w-0 flex-1 rounded-fc-sm focus-visible:outline-2 focus-visible:outline-fc-ring">
			<span class="block truncate text-fc-md font-medium text-fc-fg">{workbook.title}</span>
			<span class="block truncate text-fc-sm text-fc-fg-muted">{sheets} · {when}</span>
		</a>
	{:else}
		<div class="min-w-0 flex-1">
			<span class="block truncate text-fc-md font-medium text-fc-fg">{workbook.title}</span>
			<span class="block truncate text-fc-sm text-fc-fg-muted">{sheets} · {when}</span>
		</div>
	{/if}
	<Menu label={`Actions for ${workbook.title}`} items={actions} align="end">
		{#snippet trigger()}
			<Icon icon={icons.menu} size={18} />
		{/snippet}
	</Menu>
</li>
