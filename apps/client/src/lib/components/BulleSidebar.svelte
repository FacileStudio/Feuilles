<script lang="ts">
	import { Icon, IconButton } from '@facile/muse';
	import { untrack } from 'svelte';
	import { bulleMessage, embedUrl, trustedOrigin, type PageContext } from '$lib/bulle';
	import { icons } from '$lib/icons';

	let {
		open = $bindable(),
		page,
		origin,
		assistant,
		onready
	}: {
		open: boolean;
		page: PageContext;
		/** The Bulle origin, from the API's `BULLE_URL`. */
		origin: string;
		assistant: string;
		/** Bulle named the assistant once its frame loaded. */
		onready?: (name: string) => void;
	} = $props();

	let frame = $state<HTMLIFrameElement | null>(null);
	let shown = $state(false);

	const src = $derived.by(() => {
		const id = page.id;
		const bulle = origin;
		return untrack(() => embedUrl({ ...page, id }, bulle));
	});
	const dev = import.meta.env.DEV;

	$effect(() => {
		if (open) shown = true;
	});

	function onmessage(event: MessageEvent) {
		if (!trustedOrigin(event.origin, origin, dev) || event.source !== frame?.contentWindow) return;
		const message = bulleMessage(event.data);
		if (message?.type === 'bulle:close') open = false;
		if (message?.type === 'bulle:ready') onready?.(message.assistant.name);
	}
</script>

<svelte:window {onmessage} />

{#if shown}
	<aside
		class="fixed inset-0 z-40 flex flex-col bg-fc-bg shadow-lg lg:inset-y-0 lg:right-0 lg:left-auto lg:w-[400px]"
		class:hidden={!open}
		aria-label={assistant}
		data-testid="bulle-sidebar"
	>
		<div class="flex h-12 shrink-0 items-center justify-end px-2">
			<IconButton variant="ghost" aria-label="Close {assistant}" onclick={() => (open = false)}>
				<Icon icon={icons.close} size={18} />
			</IconButton>
		</div>
		<iframe bind:this={frame} title={assistant} {src} class="min-h-0 w-full flex-1 bg-fc-bg" allow="clipboard-write"
		></iframe>
	</aside>
{/if}
