<script lang="ts">
	import { page } from '$app/state';
	import { Alert, ApprovalCard, Button, EmptyState, PageHeader, Spinner, StatusDot, toast } from '@facile/muse';
	import { ApiError, backend, type PendingAction } from '$lib/backend';
	import { icons } from '$lib/icons';
	import { STATES, targetArgs, targetWorkbook } from '$lib/pending';
	import { signIn } from '$lib/state/account.svelte';
	import { explain } from '$lib/workbooks/list.svelte';

	const id = $derived(page.url.searchParams.get('action') ?? '');

	let action = $state<PendingAction | null | undefined>(undefined);
	let outcome = $state<'confirmed' | 'dismissed' | null>(null);
	let busy = $state(false);
	let denied = $state(false);

	const framed = window.top !== window.self;

	function failed(error: unknown) {
		if (error instanceof ApiError && error.code === 'permission_denied') denied = true;
		else toast.danger(explain(error));
	}

	$effect(() => {
		const wanted = id;
		action = undefined;
		outcome = null;
		denied = false;
		if (framed) return;
		backend
			.actions()
			.then((list) => (action = list.find((item) => item.id === wanted) ?? null))
			.catch((error) => {
				action = null;
				failed(error);
			});
	});

	const workbookId = $derived(action ? targetWorkbook(action) : null);
	const status = $derived(action ? (STATES[action.state] ?? STATES.missing) : null);
	const approval = $derived(
		action ? { id: action.id, tool: action.kind, summary: action.summary, args: targetArgs(action) } : null
	);
	const expires = $derived(action ? new Date(action.expires_at).toLocaleString() : '');

	async function answer(confirm: boolean) {
		if (!action) return;
		busy = true;
		try {
			if (confirm) await backend.confirmAction(action.id);
			else await backend.dismissAction(action.id);
			outcome = confirm ? 'confirmed' : 'dismissed';
		} catch (error) {
			failed(error);
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Agent request · Feuilles</title></svelte:head>

<div class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-4 py-6 sm:px-6 md:py-10">
	<PageHeader title="Agent request" description="An agent wants to remove cells and needs your agreement first." />
	{#if framed}
		<Alert tone="warning" title="Open this request in Feuilles">
			<p>An agent request can only be answered in a Feuilles tab of its own, never inside another page.</p>
			<a class="mt-2 inline-block underline" href={page.url.href} target="_blank" rel="noopener">Open it in a new tab</a>
		</Alert>
	{:else if denied}
		<Alert tone="warning" title="Sign in again in your browser">
			<p>
				Only a session you signed in to in this browser can confirm an agent's request. The mobile and desktop apps,
				the CLI and sessions from before this update cannot confirm.
			</p>
			<Button class="mt-3" size="sm" onclick={() => signIn(page.url.pathname + page.url.search)}>Sign in again</Button>
		</Alert>
	{:else if action === undefined}
		<div class="flex justify-center py-16"><Spinner /></div>
	{:else if outcome}
		<EmptyState
			icon={outcome === 'confirmed' ? icons.check : icons.close}
			title={outcome === 'confirmed' ? 'Done' : 'Request declined'}
			description={outcome === 'confirmed' ? 'The change reached everyone who has the workbook open.' : 'Nothing changed.'}
		>
			{#if workbookId && outcome === 'confirmed'}<Button href="/w/{workbookId}">Open the workbook</Button>{/if}
		</EmptyState>
	{:else if !action || !approval || !status}
		<EmptyState
			icon={icons.warning}
			title="Request not found"
			description="It was already confirmed, declined or expired, or it is not yours."
		>
			<Button href="/">Back to workbooks</Button>
		</EmptyState>
	{:else}
		<div class="flex flex-wrap items-center gap-3 text-fc-xs text-fc-fg-muted">
			<StatusDot tone={status.tone} label={status.label} />
			<span>Client {action.client_id}</span>
			<span>Expires {expires}</span>
			{#if workbookId}<a class="text-fc-fg underline" href="/w/{workbookId}">See the workbook now</a>{/if}
		</div>
		<ApprovalCard {approval} {busy} onApprove={() => answer(true)} onDecline={() => answer(false)} />
	{/if}
</div>
