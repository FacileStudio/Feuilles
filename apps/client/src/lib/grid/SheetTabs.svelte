<script lang="ts">
	import { Icon, IconButton, Menu, type MenuEntry } from '@facile/muse';
	import type { Session } from '$lib/engine/session.svelte';
	import { icons } from '$lib/icons';
	import type { GridCommands } from './commands';

	let { session, commands }: { session: Session; commands: GridCommands } = $props();

	let renaming = $state<number | null>(null);
	let draft = $state('');

	const sheets = $derived.by(() => {
		void session.revision;
		const view = session.view;
		if (!view) return { list: [], selected: 0 };
		const list = view.getWorksheetsProperties().map((sheet, index) => ({ ...sheet, index }));
		return { list: list.filter((sheet) => sheet.state === 'visible'), selected: view.getSelectedSheet() };
	});
	const off = $derived(!session.canEdit);

	function startRename(index: number, name: string) {
		if (off) return;
		renaming = index;
		draft = name;
	}

	function finishRename(save: boolean) {
		const index = renaming;
		renaming = null;
		if (save && index !== null && draft.trim()) commands.sheet(index, { rename: draft.trim() });
	}

	function items(index: number, name: string): MenuEntry[] {
		const last = sheets.list.length - 1;
		return [
			{ label: 'Rename', disabled: off, onselect: () => startRename(index, name) },
			{ label: 'Move left', disabled: off || index === 0, onselect: () => commands.sheet(index, { moveTo: index - 1 }) },
			{ label: 'Move right', disabled: off || index >= last, onselect: () => commands.sheet(index, { moveTo: index + 1 }) },
			{ separator: true },
			{ label: 'Delete', tone: 'danger', disabled: off || last === 0, onselect: () => commands.sheet(index, { remove: true }) }
		];
	}
</script>

<div class="flex h-11 shrink-0 items-center gap-1 overflow-x-auto bg-fc-component px-2" role="tablist" aria-label="Sheets">
	<IconButton variant="ghost" aria-label="Add a sheet" title="Add a sheet" disabled={off} onclick={() => commands.addSheet()}>
		<Icon icon={icons.plus} />
	</IconButton>
	{#each sheets.list as sheet (sheet.sheet_id)}
		{@const active = sheet.index === sheets.selected}
		{#if renaming === sheet.index}
			<input
				class="h-8 w-32 rounded-fc-sm px-2 text-fc-sm"
				aria-label="Sheet name"
				bind:value={draft}
				onblur={() => finishRename(true)}
				onkeydown={(event) => event.key === 'Enter' ? finishRename(true) : event.key === 'Escape' && finishRename(false)}
				{@attach (node) => node.focus()}
			/>
		{:else}
			<div class="flex shrink-0 items-center rounded-fc-sm {active ? 'bg-fc-bg' : ''}">
				<button
					type="button"
					role="tab"
					aria-selected={active}
					class="h-8 px-3 text-fc-sm whitespace-nowrap {active ? 'font-medium text-fc-fg' : 'text-fc-fg-muted hover:text-fc-fg'}"
					onclick={() => commands.select((model) => model.setSelectedSheet(sheet.index))}
					ondblclick={() => startRename(sheet.index, sheet.name)}
				>
					{sheet.name}
				</button>
				{#if active}
					<Menu label={`Sheet ${sheet.name}`} items={items(sheet.index, sheet.name)} class="[&>button]:h-8 [&>button]:min-h-0 [&>button]:bg-transparent [&>button]:px-1">
						{#snippet trigger()}<Icon icon={icons.chevronDown} size={14} />{/snippet}
					</Menu>
				{/if}
			</div>
		{/if}
	{/each}
</div>
