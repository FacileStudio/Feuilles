<script lang="ts">
	import { Icon, Menu, type MenuEntry } from '@facile/muse';
	import type { Workbook } from '$lib/backend';
	import { icons } from '$lib/icons';
	import { ago } from './format';

	let {
		workbook,
		actions,
		href,
		oncontextmenu
	}: {
		workbook: Workbook;
		actions: MenuEntry[];
		href?: string;
		oncontextmenu?: (e: MouseEvent) => void;
	} = $props();

	const count = $derived(workbook.sheets.length);
	const when = $derived(workbook.trashed_at ? `Trashed ${ago(workbook.trashed_at)}` : `Edited ${ago(workbook.updated_at)}`);
	const detail = $derived(count === 0 ? when : `${count === 1 ? '1 sheet' : `${count} sheets`} · ${when}`);
</script>

<li
	class="flex min-h-14 items-center gap-3 rounded-fc-md bg-fc-component px-4 py-2 transition-colors hover:bg-fc-surface"
	{oncontextmenu}
>
	<Icon icon={icons.workbook} size={20} class="text-fc-fg-muted" />
	{#if href}
		<a {href} class="min-w-0 flex-1 rounded-fc-sm focus-visible:outline-2 focus-visible:outline-fc-ring">
			<span class="block truncate text-fc-md font-medium text-fc-fg">{workbook.title}</span>
			<span class="block truncate text-fc-sm text-fc-fg-muted">{detail}</span>
		</a>
	{:else}
		<div class="min-w-0 flex-1">
			<span class="block truncate text-fc-md font-medium text-fc-fg">{workbook.title}</span>
			<span class="block truncate text-fc-sm text-fc-fg-muted">{detail}</span>
		</div>
	{/if}
	<Menu label={`Actions for ${workbook.title}`} items={actions} align="end">
		{#snippet trigger()}
			<Icon icon={icons.more} size={18} />
		{/snippet}
	</Menu>
</li>
