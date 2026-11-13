<script lang="ts">
	import { tick } from 'svelte';
	import { Menu, type MenuEntry } from '@facile/muse';

	let {
		label,
		items,
		returnFocus
	}: {
		label: string;
		items: MenuEntry[];
		returnFocus: () => HTMLElement | null | undefined;
	} = $props();

	let point = $state({ x: 0, y: 0 });
	let anchor: HTMLDivElement;

	/** Opens the menu at a client point. Closing it hands focus to `returnFocus`. */
	export async function open(x: number, y: number) {
		point = { x, y };
		await tick();
		anchor.querySelector('button')?.click();
	}

	function redirect(event: FocusEvent) {
		if (event.target === anchor.querySelector('button')) returnFocus()?.focus();
	}
</script>

<div bind:this={anchor} class="fixed z-50 size-0 overflow-visible" style:left="{point.x}px" style:top="{point.y}px" onfocusin={redirect}>
	<Menu
		{label}
		{items}
		class="[&>span>button]:size-0 [&>span>button]:min-h-0 [&>span>button]:min-w-0 [&>span>button]:overflow-hidden [&>span>button]:p-0 [&>span>button]:opacity-0 [&>span>[role=tooltip]]:hidden"
	>
		{#snippet trigger()}<span class="block size-0"></span>{/snippet}
	</Menu>
</div>
