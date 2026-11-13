<script lang="ts">
	import type { MenuEntry } from '@facile/muse';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import type { Session } from '$lib/engine/session.svelte';
	import type { GridCommands } from './commands';
	import type { Zone } from './pointer';
	import { menuCopy, menuPaste } from './grid-clipboard';
	import { icons } from '$lib/icons';

	let { session, commands, grid }: { session: Session; commands: GridCommands; grid: () => HTMLElement | null } = $props();

	let zone = $state<Zone>('cells');
	let menu: ContextMenu;

	/** Opens the menu at a client point for the part of the sheet that was clicked. */
	export async function open(x: number, y: number, at: Zone) {
		zone = at;
		await menu.open(x, y);
	}

	const off = $derived(!session.canEdit);
	const items = $derived.by<MenuEntry[]>(() => {
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

<ContextMenu bind:this={menu} label="Cell actions" {items} returnFocus={grid} />
