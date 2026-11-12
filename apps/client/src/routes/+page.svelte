<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button, Dropzone, EmptyState, PageHeader, Spinner, toast, type MenuEntry } from '@facile/muse';
	import { backend, exportUrl, type Workbook } from '$lib/backend';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import { icons } from '$lib/icons';
	import { spaces } from '$lib/state/spaces.svelte';
	import RenameModal from '$lib/workbooks/RenameModal.svelte';
	import WorkbookRow from '$lib/workbooks/WorkbookRow.svelte';
	import { titleFromFile } from '$lib/workbooks/format';
	import { explain, WorkbookList } from '$lib/workbooks/list.svelte';

	const MAX_IMPORT = 20 * 1024 * 1024;
	const list = new WorkbookList(false);
	let busy = $state(false);
	let renaming = $state<Workbook | null>(null);
	let renameOpen = $state(false);
	let contextMenu = $state<{ x: number; y: number; workbook: Workbook } | null>(null);

	$effect(() => {
		if (spaces.loaded) void list.load(spaces.currentId);
	});

	async function create() {
		busy = true;
		try {
			const workbook = await backend.create('Untitled workbook', spaces.currentId);
			await goto(`/w/${workbook.id}`);
		} catch (error) {
			toast.danger(explain(error));
		} finally {
			busy = false;
		}
	}

	async function importFiles(files: File[]) {
		const file = files[0];
		if (!file) return;
		busy = true;
		try {
			const workbook = await backend.import(file, spaces.currentId);
			toast.success(`Imported ${titleFromFile(file.name)}`);
			await goto(`/w/${workbook.id}`);
		} catch (error) {
			toast.danger(explain(error));
		} finally {
			busy = false;
		}
	}

	function actions(workbook: Workbook): MenuEntry[] {
		return [
			{ label: 'Open', icon: icons.external, href: `/w/${workbook.id}` },
			{ label: 'Rename', icon: icons.edit, onselect: () => ((renaming = workbook), (renameOpen = true)) },
			{ label: 'Download .xlsx', icon: icons.download, href: exportUrl(workbook.id, 'xlsx') },
			{ separator: true },
			{
				label: 'Move to trash',
				icon: icons.remove,
				tone: 'danger',
				onselect: () => list.act(workbook.id, () => backend.trash(workbook.id), 'Moved to trash')
			}
		];
	}

	async function rename(title: string) {
		const target = renaming;
		if (target) await list.act(target.id, () => backend.rename(target.id, title), 'Renamed');
	}
</script>

<PageHeader title="Workbooks" description="Spreadsheets in this space, newest first.">
	{#snippet actions()}
		<Button icon={icons.plus} onclick={create} disabled={busy}>New workbook</Button>
	{/snippet}
</PageHeader>

<Dropzone
	accept=".xlsx,.csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,text/csv"
	maxSize={MAX_IMPORT}
	maxFiles={1}
	disabled={busy}
	label="Import an .xlsx or .csv file"
	browseLabel="Browse"
	dropAnnouncement="Drop to import the file"
	hint="Up to 20 MB. The file becomes a new workbook."
	onFiles={importFiles}
	onReject={() => toast.danger('Only one .xlsx or .csv file of at most 20 MB')}
/>

{#if list.loading}
	<div class="flex justify-center py-12"><Spinner /></div>
{:else if list.error}
	<EmptyState icon={icons.warning} title="The workbooks could not be loaded" description={list.error}>
		<Button variant="secondary" onclick={() => list.load(spaces.currentId)}>Try again</Button>
	</EmptyState>
{:else if list.items.length === 0}
	<EmptyState icon={icons.workbook} title="No workbook yet" description="Create one, or import an .xlsx or .csv file above.">
		<Button icon={icons.plus} onclick={create} disabled={busy}>New workbook</Button>
	</EmptyState>
{:else}
	<ul class="flex flex-col gap-2">
		{#each list.items as workbook (workbook.id)}
			<WorkbookRow
				{workbook}
				href={`/w/${workbook.id}`}
				actions={actions(workbook)}
				oncontextmenu={(e) => {
					e.preventDefault();
					contextMenu = { x: e.clientX, y: e.clientY, workbook };
				}}
			/>
		{/each}
	</ul>
{/if}

<RenameModal bind:open={renameOpen} title={renaming?.title ?? ''} onSave={rename} />

{#if contextMenu}
	<ContextMenu
		x={contextMenu.x}
		y={contextMenu.y}
		items={actions(contextMenu.workbook)}
		onclose={() => (contextMenu = null)}
	/>
{/if}
