<script lang="ts">
	import { tick } from 'svelte';
	import { Menu, type MenuEntry } from '@facile/muse';
	import type { Session } from '$lib/engine/session.svelte';
	import type { GridCommands } from './commands';
	import type { Zone } from './pointer';
	import { menuCopy, menuPaste } from './grid-clipboard';
	import { icons } from '$lib/icons';

	let { session, commands, grid }: { session: Session; commands: GridCommands; grid: () => HTMLElement | null } = $props();

	let position = $state<{ x: number; y: number; zone: Zone } | null>(null);
	let anchor: HTMLDivElement;

	/** Opens the menu at a client point for the part of the sheet that was clicked. */
	export async function open(x: number, y: number, zone: Zone) {
		position = { x, y, zone };
		await tick();
		anchor.querySelector('button')?.click();
	}

	const off = $derived(!session.canEdit);
	const items = $derived.by<MenuEntry[]>(() => {
		const zone = position?.zone ?? 'cells';
		const rows: MenuEntry[] = [
			{ label: 'Insert row above', icon: icons.insertAbove, disabled: off, onselect: () => commands.insertRows(false) },
			{ label: 'Insert row below', icon: icons.insertBelow, disabled: off, onselect: () => commands.insertRows(true) },
			{ label: 'Delete rows', icon: icons.remove, tone: 'danger', disabled: off, onselect: () => commands.deleteLines(false) }
		];
		const columns: MenuEntry[] = [
			{ label: 'Insert column left', icon: icons.insertLeft, disabled: off, onselect: () => commands.insertColumns(false) },
			{ label: 'Insert column right', icon: icons.insertRight, disabled: off, onselect: () => commands.insertColumns(true) },
			{ label: 'Delete columns', icon: icons.remove, tone: 'danger', disabled: off, onselect: () => commands.deleteLines(true) }
		];
		if (zone === 'rows') return rows;
		if (zone === 'columns') return columns;
		return [
			{ label: 'Copy', icon: icons.copy, onselect: () => menuCopy(grid()) },
			{ label: 'Cut', icon: icons.cut, disabled: off, onselect: () => menuCopy(grid(), true) },
			{ label: 'Paste', icon: icons.paste, disabled: off, onselect: () => void menuPaste(session) },
			{ separator: true },
			...rows.slice(0, 2),
			...columns.slice(0, 2),
			{ separator: true },
			rows[2],
			columns[2],
			{ label: 'Clear contents', icon: icons.clear, disabled: off, onselect: () => commands.clear() }
		];
	});
</script>

<div
	bind:this={anchor}
	class="fixed z-50 size-0 overflow-visible"
	style:left="{position?.x ?? 0}px"
	style:top="{position?.y ?? 0}px"
>
	<Menu label="Cell actions" {items} class="[&>button]:size-0 [&>button]:min-h-0 [&>button]:min-w-0 [&>button]:overflow-hidden [&>button]:p-0 [&>button]:opacity-0">
		{#snippet trigger()}<span class="block size-0"></span>{/snippet}
	</Menu>
</div>
