<script lang="ts">
	import {
		FeatureSplit,
		ForDevelopers,
		LANDING_HEADING,
		LandingCta,
		LandingFooter,
		LandingHeader,
		PricingPlans,
		SignInWithFacile,
		WorksWith,
		signUpUrl,
		type DeveloperRow,
		type FeatureItem,
		type WorksWithApp
	} from '@facile/muse';
	import { signInUrl } from '$lib/backend';
	import { locale, rememberLocale, t } from '$lib/i18n/index.svelte';
	import { icons } from '$lib/icons';
	import BulleTile from './BulleTile.svelte';
	import EmbedTile from './EmbedTile.svelte';
	import ImportTile from './ImportTile.svelte';
	import SheetDemo from './SheetDemo.svelte';

	const features = $derived<FeatureItem[]>(
		(['live', 'formulas', 'files', 'space', 'format', 'trash'] as const).map((key, index) => ({
			icon: [icons.usersGroup, icons.workbook, icons.download, icons.folder, icons.fill, icons.refresh][index],
			title: t(`landing.feature.${key}.title`),
			text: t(`landing.feature.${key}.text`)
		}))
	);

	const apps = $derived<WorksWithApp[]>([
		{ name: 'Docs', icon: icons.docs, line: t('landing.with.docs'), href: 'https://docs.facile.studio' },
		{ name: 'Toile', icon: icons.toile, line: t('landing.with.toile'), href: 'https://toile.facile.studio' },
		{ name: 'Bulle', icon: icons.bulle, line: t('landing.with.bulle'), href: 'https://bulle.facile.studio' }
	]);

	const rows = $derived<DeveloperRow[]>([
		{ kind: 'mcp', text: t('landing.dev.mcp') },
		{ kind: 'api', text: t('landing.dev.api'), href: 'https://feuilles.facile.studio/docs' },
		{ kind: 'source', text: t('landing.dev.source'), href: 'https://github.com/FacileStudio/Feuilles' }
	]);

	const links = $derived([
		{ href: '#features', label: t('landing.nav.features') },
		{ href: '#pricing', label: t('landing.nav.pricing') }
	]);
	const columns = $derived([
		{ title: 'Feuilles', links },
		{
			title: 'Facile',
			links: [
				{ href: 'https://suite.facile.studio', label: t('landing.nav.suite'), external: true },
				{ href: signUpUrl(locale()), label: t('landing.nav.signup'), external: true }
			]
		}
	]);
	$effect(() => {
		document.querySelector('meta[name="description"]')?.setAttribute('content', t('landing.meta'));
	});

	const container = 'mx-auto w-full max-w-fc-xl px-4 sm:px-6 md:px-10';
	const body = 'w-full max-w-[960px]';
</script>

<svelte:head>
	<title>{t('landing.title')}</title>
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
				<h1 class="font-fc-title text-fc-2xl font-bold text-balance sm:text-fc-3xl">{t('landing.slogan')}</h1>
				<p class="text-fc-md text-balance text-fc-fg-muted sm:text-fc-lg">{t('landing.subtitle')}</p>
			</div>
			<LandingCta signInHref={signInUrl} />
			<SheetDemo />
		</section>

		<section aria-labelledby="tiles-title" class="{container} flex flex-col gap-8">
			<h2 id="tiles-title" class={LANDING_HEADING}>{t('landing.tiles')}</h2>
			<div class="grid {body} gap-6 md:grid-cols-3">
				<ImportTile />
				<EmbedTile />
				<BulleTile />
			</div>
		</section>

		<div class={container}>
			<FeatureSplit id="features" class="{body} scroll-mt-24" title={t('landing.features')} description={t('landing.featuresLine')} {features} />
		</div>

		<div class={container}>
			<WorksWith class={body} {apps} description={t('landing.with.line')} />
		</div>

		<section class="{container} flex flex-col items-center gap-5 text-center">
			<h2 class={LANDING_HEADING}>{t('landing.cta.title')}</h2>
			<p class="text-fc-md text-fc-fg-muted">{t('landing.cta.line')}</p>
			<LandingCta signInHref={signInUrl} align="center" class="w-full" />
		</section>

		<div class={container}>
			<PricingPlans
				id="pricing"
				class="{body} scroll-mt-24"
				title={t('landing.nav.pricing')}
				headingClass={LANDING_HEADING}
				highlight={['ai.access']}
				highlightOnly
				notes={{ '*': t('landing.plan.paid'), solo: t('landing.plan.included') }}
				recommended="pro"
				suiteHref="https://suite.facile.studio/#tarifs"
			/>
		</div>

		<div class={container}>
			<ForDevelopers class={body} {rows} description={t('landing.dev.line')} selfHostHref={`https://suite.facile.studio/${locale() === 'fr' ? '' : `${locale()}/`}auto-hebergement`} />
		</div>
	</main>

	<LandingFooter appName="Feuilles" icon={icons.logo} tagline={t('landing.line')} {columns} legal={{ onLocaleChange: rememberLocale }} />
</div>
