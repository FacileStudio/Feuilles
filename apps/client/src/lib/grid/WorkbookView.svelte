<script lang="ts">
	import type { Session } from '$lib/engine/session.svelte';
	import ContextMenu from './ContextMenu.svelte';
	import FormulaBar from './FormulaBar.svelte';
	import Grid from './Grid.svelte';
	import SheetTabs from './SheetTabs.svelte';
	import Toolbar from './Toolbar.svelte';
	import { GridCommands } from './commands';
	import { CellEdit } from './editing.svelte';

	let { session }: { session: Session } = $props();

	const commands = $derived(new GridCommands(session));
	const edit = new CellEdit();
	let menu: ContextMenu;
	let host: HTMLDivElement;

	const gridElement = () => host?.querySelector<HTMLElement>('.feuilles-grid') ?? null;
</script>

<div bind:this={host} class="flex min-h-0 flex-1 flex-col">
	<div class="hidden md:contents">
		<Toolbar {session} {commands} />
		<FormulaBar {session} {commands} {edit} focusGrid={() => gridElement()?.focus()} />
	</div>
	<Grid {session} {commands} {edit} onMenu={(x, y, zone) => menu.open(x, y, zone)} />
	<SheetTabs {session} {commands} />
	<ContextMenu bind:this={menu} {session} {commands} grid={gridElement} />
</div>
