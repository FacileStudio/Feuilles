<script lang="ts">
	import { Button, Card, Icon, LandingFooter, LandingHeader, SignInWithFacile } from '@facile/muse';
	import { signInUrl } from '$lib/backend';
	import { icons } from '$lib/icons';
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

		<section id="features" aria-labelledby="features-title" class="{container} flex scroll-mt-24 flex-col gap-8">
			<h2 id="features-title" class="font-fc-title text-fc-xl font-bold sm:text-fc-2xl">What you get</h2>
			<ul class="grid gap-4 sm:grid-cols-2">
				{#each FEATURES as feature (feature.title)}
					<li>
						<Card class="flex h-full flex-col gap-3 p-6">
							<Icon icon={feature.icon} size={22} class="text-fc-fg-muted" />
							<h3 class="text-fc-md font-bold">{feature.title}</h3>
							<p class="text-fc-sm text-fc-fg-muted">{feature.body}</p>
						</Card>
					</li>
				{/each}
			</ul>
		</section>

		<section class="{container} flex flex-col items-start gap-5">
			<h2 class="font-fc-title text-fc-xl font-bold sm:text-fc-2xl">Start a workbook</h2>
			<p class="text-fc-md text-fc-fg-muted">Sign in with your Facile account and open a blank sheet.</p>
			<SignInWithFacile href={signInUrl} size="lg" />
		</section>
	</main>

	<LandingFooter appName="Feuilles" icon={icons.logo} tagline="The collaborative spreadsheet of Facile." {columns} legal={{ hideLocaleSwitch: true }} />
</div>
