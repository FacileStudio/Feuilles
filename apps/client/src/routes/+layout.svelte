<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { AuthScreen, Spinner, Toaster, provideLocale } from '@facile/muse';
	import AppShell from '$lib/shell/AppShell.svelte';
	import { account, loadAccount } from '$lib/state/account.svelte';
	import { restoreTheme } from '$lib/state/theme.svelte';
	import { signInUrl } from '$lib/backend';

	let { children }: { children: Snippet } = $props();

	provideLocale(() => 'en');

	$effect(() => {
		restoreTheme();
		void loadAccount();
	});
</script>

{#if !account.loaded}
	<div class="flex min-h-dvh items-center justify-center bg-fc-page">
		<Spinner />
	</div>
{:else if account.user}
	<AppShell user={account.user}>
		{@render children()}
	</AppShell>
{:else}
	<AuthScreen appName="Feuilles" href={signInUrl}>
		{#snippet tagline()}
			<p>Spreadsheets your space edits together.</p>
		{/snippet}
		{#snippet notice()}
			{#if account.error}
				<p class="text-fc-sm text-fc-danger">{account.error}</p>
			{/if}
		{/snippet}
	</AuthScreen>
{/if}

<Toaster class="max-md:pb-24" />
