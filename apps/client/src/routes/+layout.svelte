<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { AuthScreen, Icon, Spinner, Toaster, provideLocale } from '@facile/muse';
	import { page } from '$app/state';
	import Landing from '$lib/landing/Landing.svelte';
	import AppShell from '$lib/shell/AppShell.svelte';
	import { icons } from '$lib/icons';
	import { account, loadAccount } from '$lib/state/account.svelte';
	import { restoreTheme } from '$lib/state/theme.svelte';
	import { signInUrl } from '$lib/backend';
	import { bindLocale, t, visitorLocale } from '$lib/i18n/index.svelte';

	let { children }: { children: Snippet } = $props();

	const i18n = provideLocale(() => (account.user ? 'en' : visitorLocale()));
	bindLocale(() => i18n.locale);

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
{:else if page.url.pathname === '/' && !account.error}
	<Landing />
{:else}
	<AuthScreen appName="Feuilles" href={signInUrl}>
		{#snippet logo()}
			<Icon icon={icons.logo} size={32} class="text-fc-fg" />
		{/snippet}
		{#snippet tagline()}
			<p>{t('landing.slogan')}</p>
		{/snippet}
		{#snippet notice()}
			{#if account.error}
				<p class="text-fc-sm text-fc-danger">{account.error}</p>
			{/if}
		{/snippet}
	</AuthScreen>
{/if}

<Toaster class="max-md:pb-24" />
