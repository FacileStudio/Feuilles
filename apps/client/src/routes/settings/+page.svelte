<script lang="ts">
	import { page } from '$app/state';
	import { PreferencesSection, ProfileSection, SettingsLayout, SettingsSection } from '@facile/muse';
	import { backend, type ServerVersion } from '$lib/backend';
	import { ENGINE_VERSION } from '$lib/engine/version';
	import { icons } from '$lib/icons';
	import { account } from '$lib/state/account.svelte';
	import { setTheme, theme } from '$lib/state/theme.svelte';

	const sections = [
		{ id: 'profile', label: 'Profile', icon: icons.userCircle, href: '/settings?section=profile' },
		{ id: 'preferences', label: 'Preferences', icon: icons.palette, href: '/settings?section=preferences' },
		{ id: 'advanced', label: 'Advanced', icon: icons.code, href: '/settings?section=advanced' }
	];

	const active = $derived(page.url.searchParams.get('section') ?? undefined);
	let server = $state<ServerVersion | null>(null);

	$effect(() => {
		void backend.version().then((answer) => (server = answer)).catch(() => (server = null));
	});
</script>

<SettingsLayout {sections} {active} backHref="/settings" description="Your Facile profile and how Feuilles looks.">
	{#snippet children(section)}
		{#if section === 'profile'}
			<ProfileSection
				name={account.user?.name?.trim() || account.user?.email || ''}
				email={account.user?.email}
				avatar={account.user?.avatar_url || undefined}
				manageHref="https://sso.facile.studio/account"
				manageLabel="Manage on Facile"
			/>
		{:else if section === 'preferences'}
			<PreferencesSection showLanguage={false} theme={theme.mode} onThemeChange={setTheme} />
		{:else}
			<SettingsSection title="Versions" description="The engine in this page must match the server's to open a workbook.">
				<dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-fc-sm">
					<dt class="text-fc-fg-muted">Spreadsheet engine in this page</dt>
					<dd class="font-fc-mono text-fc-fg">IronCalc {ENGINE_VERSION}</dd>
					<dt class="text-fc-fg-muted">Spreadsheet engine on the server</dt>
					<dd class="font-fc-mono text-fc-fg">{server?.engine ? `IronCalc ${server.engine}` : 'Unknown'}</dd>
					<dt class="text-fc-fg-muted">Feuilles server</dt>
					<dd class="font-fc-mono text-fc-fg">{server?.version ?? 'Unknown'}</dd>
				</dl>
			</SettingsSection>
		{/if}
	{/snippet}
</SettingsLayout>
