<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { MobileNav, SideBar, SpaceSwitcher, Topbar, type Theme } from '@facile/muse';
	import { icons } from '$lib/icons';
	import type { SessionUser } from '$lib/backend';
	import { signOut } from '$lib/state/account.svelte';
	import { loadSpaces, selectSpace, spaces } from '$lib/state/spaces.svelte';
	import { setTheme, theme } from '$lib/state/theme.svelte';

	let { user, children }: { user: SessionUser; children: Snippet } = $props();

	let collapsed = $state(false);

	const isActive = (href: string) =>
		href === '/' ? page.url.pathname === '/' || page.url.pathname.startsWith('/w/') : page.url.pathname.startsWith(href);

	const pages = $derived([
		{ href: '/', label: 'Workbooks', icon: icons.workbook, active: isActive('/') },
		{ href: '/trash', label: 'Trash', icon: icons.remove, active: isActive('/trash') }
	]);
	const onSettings = $derived(isActive('/settings'));
	const fullBleed = $derived(page.url.pathname.startsWith('/w/'));
	const menuUser = $derived({ name: user.name?.trim() || user.email, email: user.email, avatar: user.avatar_url || undefined });
	const userMenu = $derived({
		settingsHref: '/settings',
		helpHref: 'https://github.com/FacileStudio/Feuilles',
		onSignOut: () => void signOut(),
		showLanguage: false,
		theme: theme.mode,
		onThemeChange: (next: Theme) => setTheme(next)
	});
	const switcher = $derived(spaces.list.map((space) => ({ id: space.id, name: space.name })));

	$effect(() => {
		void loadSpaces();
	});
</script>

<div class="flex h-dvh w-full overflow-hidden bg-fc-page">
	<div class="hidden h-full shrink-0 p-3 md:block">
		<SideBar
			icon={icons.workbook}
			title="Feuilles"
			bind:collapsed
			{pages}
			spaces={switcher}
			activeSpaceId={spaces.currentId}
			onSpaceSelect={selectSpace}
			personalSpaceLabel="Personal"
			manageSpacesHref="https://sso.facile.studio/spaces"
			user={menuUser}
			userHref="/settings"
			userActive={onSettings}
			{userMenu}
			class="h-full"
		/>
	</div>

	<main class="flex min-w-0 flex-1 flex-col overflow-auto overscroll-contain {fullBleed ? '' : 'pb-28 md:pb-0'}">
		{#if !fullBleed}
			<Topbar class="md:hidden">
				<div class="flex min-w-0 flex-1 items-center gap-3">
					<span class="text-fc-md font-semibold text-fc-fg">Feuilles</span>
					{#if switcher.length > 0}
						<div class="min-w-0 max-w-56 flex-1">
							<SpaceSwitcher
								spaces={switcher}
								activeId={spaces.currentId}
								onSelect={selectSpace}
								personalLabel="Personal"
								manageHref="https://sso.facile.studio/spaces"
							/>
						</div>
					{/if}
				</div>
			</Topbar>
			<div class="mx-auto flex w-full max-w-fc-xl flex-col gap-8 px-4 py-8 sm:px-6 md:px-10 md:py-10">
				{@render children()}
			</div>
		{:else}
			{@render children()}
		{/if}
	</main>

	{#if !fullBleed}
		<MobileNav items={pages} user={menuUser} profileHref="/settings" profileActive={onSettings} {userMenu} />
	{/if}
</div>
