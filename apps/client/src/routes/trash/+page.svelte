<script lang="ts">
	import { ConfirmModal, EmptyState, PageHeader, Spinner, type MenuEntry } from '@facile/muse';
	import { backend, type Workbook } from '$lib/backend';
	import { icons } from '$lib/icons';
	import { spaces } from '$lib/state/spaces.svelte';
	import WorkbookRow from '$lib/workbooks/WorkbookRow.svelte';
	import { WorkbookList } from '$lib/workbooks/list.svelte';

	const list = new WorkbookList(true);
	let doomed = $state<Workbook | null>(null);
	let confirmOpen = $state(false);

	$effect(() => {
		if (spaces.loaded) void list.load(spaces.currentId);
	});

	function actions(workbook: Workbook): MenuEntry[] {
		return [
			{ label: 'Restore', icon: icons.refresh, onselect: () => list.act(workbook.id, () => backend.restore(workbook.id), 'Restored') },
			{ separator: true },
			{ label: 'Delete forever', icon: icons.remove, tone: 'danger', onselect: () => ((doomed = workbook), (confirmOpen = true)) }
		];
	}

	async function destroy() {
		const target = doomed;
		if (target) await list.act(target.id, () => backend.destroy(target.id), 'Deleted forever');
		confirmOpen = false;
	}
</script>

<PageHeader title="Trash" description="Trashed workbooks are deleted for good after 30 days." />

{#if list.loading}
	<div class="flex justify-center py-12"><Spinner /></div>
{:else if list.error}
	<EmptyState icon={icons.warning} title="The trash could not be loaded" description={list.error} />
{:else if list.items.length === 0}
	<EmptyState icon={icons.remove} title="The trash is empty" description="Workbooks you move to the trash show up here." />
{:else}
	<ul class="flex flex-col gap-2">
		{#each list.items as workbook (workbook.id)}
			<WorkbookRow {workbook} actions={actions(workbook)} />
		{/each}
	</ul>
{/if}

<ConfirmModal
	bind:open={confirmOpen}
	title="Delete this workbook forever?"
	description={doomed ? `"${doomed.title}" and its history are deleted. This cannot be undone.` : undefined}
	confirmLabel="Delete forever"
	tone="danger"
	onConfirm={destroy}
	onCancel={() => (confirmOpen = false)}
/>
