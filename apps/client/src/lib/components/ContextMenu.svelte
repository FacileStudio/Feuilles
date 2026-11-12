<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Icon, type MenuAction, type MenuEntry } from '@facile/muse';

	let {
		x,
		y,
		items,
		onclose
	}: {
		x: number;
		y: number;
		items: MenuEntry[];
		onclose: () => void;
	} = $props();

	let el = $state<HTMLDivElement | null>(null);
	let pos = $state<{ left: number; top: number } | null>(null);

	$effect(() => {
		const node = el;
		if (!node) return;
		void x;
		void y;
		void tick().then(() => {
			const { width, height } = node.getBoundingClientRect();
			pos = {
				left: Math.max(8, Math.min(x, window.innerWidth - width - 8)),
				top: Math.max(8, Math.min(y, window.innerHeight - height - 8))
			};
			node.querySelector<HTMLElement>('button, a')?.focus();
		});
	});

	onMount(() => {
		const outside = (e: PointerEvent) => {
			if (el && !el.contains(e.target as Node)) onclose();
		};
		window.addEventListener('pointerdown', outside);
		return () => window.removeEventListener('pointerdown', outside);
	});

	function keydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			return onclose();
		}
		if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
		e.preventDefault();
		const focusable = [...(el?.querySelectorAll<HTMLElement>('button, a') ?? [])];
		const current = focusable.indexOf(document.activeElement as HTMLElement);
		const step = e.key === 'ArrowDown' ? 1 : -1;
		const next = (current + step + focusable.length) % focusable.length;
		focusable[next]?.focus();
	}

	function run(item: MenuAction) {
		if (item.disabled) return;
		onclose();
		item.onselect?.();
	}
</script>

<div
	bind:this={el}
	role="menu"
	tabindex="-1"
	class="fixed z-50 min-w-44 rounded-fc-md bg-fc-component p-1 shadow-lg {pos ? '' : 'invisible'}"
	style:left="{pos?.left ?? 0}px"
	style:top="{pos?.top ?? 0}px"
	onclick={(e) => e.stopPropagation()}
	oncontextmenu={(e) => e.preventDefault()}
	onkeydown={keydown}
>
	{#each items as item, i (i)}
		{#if 'separator' in item}
			<div role="separator" class="my-1 h-px bg-fc-surface"></div>
		{:else if item.href && !item.disabled}
			<a
				href={item.href}
				role="menuitem"
				onclick={() => run(item)}
				class="flex min-h-9 w-full items-center gap-2 rounded-fc-sm px-2.5 py-1.5 text-left text-fc-sm text-fc-fg transition-colors hover:bg-fc-surface focus-visible:outline-2 focus-visible:outline-fc-ring"
			>
				{#if item.icon}<Icon icon={item.icon} size={16} />{/if}
				<span class="truncate">{item.label}</span>
			</a>
		{:else}
			<button
				type="button"
				role="menuitem"
				disabled={item.disabled}
				onclick={() => run(item)}
				class="flex min-h-9 w-full items-center gap-2 rounded-fc-sm px-2.5 py-1.5 text-left text-fc-sm transition-colors hover:bg-fc-surface focus-visible:outline-2 focus-visible:outline-fc-ring cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 {item.tone === 'danger' ? 'text-fc-danger' : 'text-fc-fg'}"
			>
				{#if item.icon}<Icon icon={item.icon} size={16} />{/if}
				<span class="truncate">{item.label}</span>
			</button>
		{/if}
	{/each}
</div>
