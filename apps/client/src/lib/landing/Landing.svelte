<script lang="ts">
	import { Button, Icon, LandingFooter, LandingHeader, SignInWithFacile } from '@facile/muse';
	import { signInUrl } from '$lib/backend';
	import { icons } from '$lib/icons';
	import AgentTile from './AgentTile.svelte';
	import EmbedTile from './EmbedTile.svelte';
	import ImportTile from './ImportTile.svelte';
	import SheetDemo from './SheetDemo.svelte';

	const FEATURES = [
		{ icon: icons.usersGroup, title: 'Live co-editing', body: 'Everyone in the space sees each edit as it lands, with names on the cells.' },
		{ icon: icons.workbook, title: 'Real formulas', body: 'IronCalc computes Excel formulas in the browser and on the server.' },
		{ icon: icons.download, title: 'xlsx in and out', body: 'Import an xlsx or csv file. Download any workbook as xlsx.' },
		{ icon: icons.automatic, title: 'Open to agents', body: 'Agents read and edit your workbooks over MCP.' }
	];

	const links = [{ href: '#features', label: 'Features' }];
	const columns = [
		{ title: 'Feuilles', links },
		{
			title: 'Facile',
			links: [
				{ href: 'https://facile.studio', label: 'The suite', external: true },
				{ href: 'https://sso.facile.studio/signup', label: 'Create an account', external: true }
			]
		}
	];
	const container = 'mx-auto w-full max-w-fc-xl px-4 sm:px-6 md:px-10';
</script>

<svelte:head>
	<title>Feuilles · The collaborative spreadsheet</title>
</svelte:head>

<div class="flex min-h-dvh flex-col bg-fc-page text-fc-fg">
	<LandingHeader appName="Feuilles" icon={icons.logo} {links} skipHref="#main">
		{#snippet actions()}
			<SignInWithFacile href={signInUrl} size="sm" />
		{/snippet}
	</LandingHeader>

	<main id="main" class="flex flex-1 flex-col gap-20 pb-24 md:gap-28">
		<section class="{container} flex flex-col gap-8 pt-8 md:pt-16">
			<div class="flex max-w-fc-md flex-col gap-4">
				<h1 class="font-fc-title text-fc-2xl font-bold text-balance sm:text-fc-3xl">Spreadsheets your team edits together</h1>
				<p class="text-fc-md text-balance text-fc-fg-muted sm:text-fc-lg">Live co-editing on the IronCalc engine, with xlsx in and out.</p>
			</div>
			<div class="flex flex-wrap items-center gap-3">
				<SignInWithFacile href={signInUrl} size="lg" />
				<Button href="#features" variant="secondary" size="lg" iconRight="solar:arrow-down-linear">See features</Button>
			</div>
			<SheetDemo />
		</section>

		<section aria-labelledby="tiles-title" class="{container} flex flex-col gap-8">
			<h2 id="tiles-title" class="font-fc-title text-fc-xl font-bold sm:text-fc-2xl">Beyond the grid</h2>
			<div class="grid max-w-[960px] gap-6 md:grid-cols-3">
				<ImportTile />
				<AgentTile />
				<EmbedTile />
			</div>
		</section>

		<section id="features" aria-labelledby="features-title" class="{container} scroll-mt-24">
			<div class="grid max-w-[960px] gap-8 md:grid-cols-[2fr_3fr] md:gap-12">
				<div class="flex flex-col gap-3">
					<h2 id="features-title" class="font-fc-title text-fc-xl font-bold sm:text-fc-2xl">What you get</h2>
					<p class="text-fc-md text-fc-fg-muted">A spreadsheet that stays open to your team and your tools.</p>
				</div>
				<ul class="flex flex-col gap-6">
					{#each FEATURES as feature (feature.title)}
						<li class="flex gap-4">
							<span class="flex size-10 shrink-0 items-center justify-center rounded-fc-md bg-fc-component">
								<Icon icon={feature.icon} size={20} class="text-fc-fg-muted" />
							</span>
							<div class="flex flex-col gap-1">
								<h3 class="text-fc-md font-bold">{feature.title}</h3>
								<p class="text-fc-sm text-fc-fg-muted">{feature.body}</p>
							</div>
						</li>
					{/each}
				</ul>
			</div>
		</section>

		<section class="{container} flex flex-col items-start gap-5">
			<h2 class="font-fc-title text-fc-xl font-bold sm:text-fc-2xl">Start a workbook</h2>
			<p class="text-fc-md text-fc-fg-muted">Sign in with your Facile account and open a blank sheet.</p>
			<SignInWithFacile href={signInUrl} size="lg" />
		</section>
	</main>

	<LandingFooter appName="Feuilles" icon={icons.logo} tagline="The collaborative spreadsheet of Facile." {columns} legal={{ hideLocaleSwitch: true }} />
</div>
