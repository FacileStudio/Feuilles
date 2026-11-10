<script lang="ts">
	import { page } from '$app/state';
	import { Alert, Avatar, Badge, Button, EmptyState, Icon, Menu, Spinner, StatusDot, toast } from '@facile/muse';
	import { ApiError, backend, exportUrl, signInUrl, socketPath, type Workbook } from '$lib/backend';
	import { claimOrigin, type OriginClaim } from '$lib/engine/origin';
	import { Role } from '$lib/engine/protocol';
	import { Session } from '$lib/engine/session.svelte';
	import { socketUrl } from '$lib/engine/socket';
	import { ENGINE_VERSION, engineMatches } from '$lib/engine/version';
	import { loadEngine, modelFromBytes } from '$lib/engine/wasm';
	import WorkbookView from '$lib/grid/WorkbookView.svelte';
	import { icons } from '$lib/icons';
	import RenameModal from '$lib/workbooks/RenameModal.svelte';
	import { explain } from '$lib/workbooks/list.svelte';
	import { noticeText, statusLabel } from '$lib/workbooks/status';

	type Phase =
		| { kind: 'loading' }
		| { kind: 'failed'; message: string }
		| { kind: 'no_access' }
		| { kind: 'mismatch'; server: string }
		| { kind: 'ready' };

	let phase = $state<Phase>({ kind: 'loading' });
	let workbook = $state<Workbook | null>(null);
	let session = $state.raw<Session | null>(null);
	let renameOpen = $state(false);

	async function open(id: string, claim: Promise<OriginClaim>): Promise<Session | null> {
		const [meta, server] = await Promise.all([backend.get(id), backend.version().catch(() => null), loadEngine()]);
		workbook = meta;
		const stored = meta.engine_version ?? server?.engine;
		if (!engineMatches(stored, ENGINE_VERSION)) {
			phase = { kind: 'mismatch', server: stored ?? 'unknown' };
			return null;
		}
		phase = { kind: 'ready' };
		const url = socketUrl(socketPath(id), window.location);
		const { origin } = await claim;
		return new Session({ url, origin, load: (bytes) => modelFromBytes(bytes, meta.language) });
	}

	function failed(error: unknown): Phase {
		if (error instanceof ApiError && error.code === 'not_found') return { kind: 'no_access' };
		return { kind: 'failed', message: explain(error) };
	}

	$effect(() => {
		const id = page.params.id ?? '';
		let current: Session | null = null;
		let gone = false;
		const claim = claimOrigin(id);
		phase = { kind: 'loading' };
		open(id, claim)
			.then((next) => (gone ? next?.close() : (session = current = next)))
			.catch((error) => (phase = failed(error)));
		return () => {
			gone = true;
			current?.close();
			session = null;
			void claim.then(({ release }) => release());
		};
	});

	$effect(() => {
		const notice = session?.notice;
		if (!session || !notice) return;
		toast.warning(noticeText(notice));
		session.notice = null;
	});

	async function rename(title: string) {
		if (!workbook) return;
		workbook = await backend.rename(workbook.id, title).catch((error) => {
			toast.danger(explain(error));
			return workbook;
		});
	}

	const downloads = $derived(
		workbook
			? [
					{ label: 'Excel workbook (.xlsx)', icon: icons.download, href: exportUrl(workbook.id, 'xlsx') },
					{ label: 'This sheet as CSV', icon: icons.download, href: exportUrl(workbook.id, 'csv', session?.view?.getSelectedSheet() ?? 0) }
				]
			: []
	);
	const status = $derived(session ? statusLabel(session.status) : null);
</script>

<svelte:head><title>{workbook?.title ?? 'Workbook'} · Feuilles</title></svelte:head>

<div class="flex h-full min-h-0 flex-col">
	<header class="flex min-h-14 shrink-0 items-center gap-3 px-3 py-2">
		<Button variant="ghost" size="sm" icon={icons.chevronLeft} href="/">Workbooks</Button>
		<button
			type="button"
			class="min-w-0 truncate rounded-fc-sm px-2 py-1 text-left text-fc-lg font-semibold text-fc-fg hover:bg-fc-surface"
			disabled={!session?.canEdit}
			onclick={() => (renameOpen = true)}
		>
			{workbook?.title ?? ''}
		</button>
		{#if session?.role === Role.Viewer}<Badge>View only</Badge>{/if}
		<div class="ml-auto flex items-center gap-3">
			{#if session}
				<div class="flex -space-x-2" aria-label="People here">
					{#each session.presence as person (person.client_id)}
						<Avatar src={person.picture || undefined} name={person.name} size="sm" title={person.name} />
					{/each}
				</div>
			{/if}
			{#if status}<StatusDot tone={status.tone} label={status.label} pulse={status.tone === 'warning'} />{/if}
			<Menu label="Download" items={downloads} align="end">
				{#snippet trigger()}<Icon icon={icons.download} size={18} />{/snippet}
			</Menu>
		</div>
	</header>

	{#if phase.kind === 'loading'}
		<div class="flex flex-1 items-center justify-center"><Spinner /></div>
	{:else if phase.kind === 'failed'}
		<EmptyState icon={icons.warning} title="This workbook could not be opened" description={phase.message}>
			<Button href="/" variant="secondary">Back to workbooks</Button>
		</EmptyState>
	{:else if phase.kind === 'no_access'}
		<EmptyState
			icon={icons.warning}
			title="No access"
			description="This workbook does not exist, or it belongs to a space you are not a member of."
		>
			<Button href="/" variant="secondary">Back to workbooks</Button>
		</EmptyState>
	{:else if phase.kind === 'mismatch'}
		<EmptyState
			icon={icons.refresh}
			title="Feuilles was updated"
			description={`This page runs spreadsheet engine ${ENGINE_VERSION} and the workbook needs ${phase.server}. Reload the page to get the new version.`}
		>
			<Button icon={icons.refresh} onclick={() => window.location.reload()}>Reload the page</Button>
		</EmptyState>
	{:else if session}
		{#if session.status === 'revoked' && session.revokedReason === 'signed_out'}
			<Alert tone="warning" class="mx-3 mb-2">
				Your session ended. Your last unsaved edits were not kept.
				<Button size="sm" variant="secondary" href={signInUrl}>Sign in again</Button>
			</Alert>
		{:else if session.status === 'revoked'}
			<Alert tone="danger" class="mx-3 mb-2">No access: you can no longer open this workbook. Your last unsaved edits were not kept.</Alert>
		{/if}
		{#if !session.view}
			<div class="flex flex-1 items-center justify-center"><Spinner /></div>
		{:else}
			{#key session}
				<WorkbookView {session} />
			{/key}
		{/if}
	{/if}
</div>

<RenameModal bind:open={renameOpen} title={workbook?.title ?? ''} onSave={rename} />
