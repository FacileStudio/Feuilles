<script lang="ts">
	import { page } from '$app/state';
	import { Button, EmptyState } from '@facile/muse';
	import { icons } from '$lib/icons';

	const missing = $derived(page.status === 404);
</script>

<svelte:head><title>{missing ? 'Page not found' : 'Error'} · Feuilles</title></svelte:head>

<EmptyState
	icon={icons.warning}
	title={missing ? 'Page not found' : 'Something went wrong'}
	description={missing ? 'This address does not lead to a page of Feuilles.' : (page.error?.message ?? 'The page could not be shown.')}
>
	<Button href="/" variant="secondary">Back to workbooks</Button>
</EmptyState>
