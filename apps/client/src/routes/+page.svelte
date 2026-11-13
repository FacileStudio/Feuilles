<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		Button,
		Card,
		Dropzone,
		EmptyState,
		Icon,
		Menu,
		PageHeader,
		Spinner,
		toast,
		type MenuEntry
	} from '@facile/muse';
	import { backend, exportUrl, type Workbook } from '$lib/backend';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import { icons } from '$lib/icons';
	import { spaces } from '$lib/state/spaces.svelte';
	import RenameModal from '$lib/workbooks/RenameModal.svelte';
	import WorkbookPreview from '$lib/workbooks/WorkbookPreview.svelte';
	import { ago, sheetLabel, titleFromFile } from '$lib/workbooks/format';
	import { explain, WorkbookList } from '$lib/workbooks/list.svelte';

	const MAX_IMPORT = 20 * 1024 * 1024;
	const list = new WorkbookList(false);
	let busy = $state(false);
	let renaming = $state<Workbook | null>(null);
	let renameOpen = $state(false);
	let menuTarget = $state<Workbook | null>(null);
	let menuReturn: HTMLElement | null = null;
	let contextMenu: ContextMenu;

	function openMenu(event: MouseEvent, workbook: Workbook) {
		event.preventDefault();
		menuTarget = workbook;
		menuReturn = (event.currentTarget as HTMLElement).querySelector('a, [role="button"], button');
		void contextMenu.open(event.clientX, event.clientY);
	}
	let gridEl = $state<HTMLElement | null>(null);
	let cols = $state(1);

	function updateCols() {
		if (!gridEl) return;
		const style = window.getComputedStyle(gridEl).getPropertyValue('grid-template-columns');
		cols = style ? style.split(' ').filter(Boolean).length : 1;
	}

	$effect(() => {
		if (spaces.loaded) void list.load(spaces.currentId);
	});

	$effect(() => {
		if (!gridEl) return;
		updateCols();
		const ro = new ResizeObserver(updateCols);
		ro.observe(gridEl);
		return () => ro.disconnect();
	});

	const showAddCard = $derived(cols > 1 && list.items.length % cols !== 0);

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

<div class="flex flex-col gap-8 pb-24 md:pb-32">
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
		<ul
			bind:this={gridEl}
			class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
			aria-label="Workbooks"
		>
			{#each list.items as workbook (workbook.id)}
				<li
					class="group/workbook relative"
					oncontextmenu={(event) => openMenu(event, workbook)}
				>
					<Card
						href="/w/{workbook.id}"
						class="flex h-full flex-col gap-3 p-3 transition-[background-color,translate] duration-300 hover:-translate-y-1 motion-reduce:transition-none cursor-pointer"
					>
						<WorkbookPreview {workbook} />
						<span class="flex min-w-0 flex-col gap-1 px-1 pb-1">
							<span class="truncate text-fc-sm font-medium text-fc-fg">{workbook.title}</span>
							<span class="text-fc-xs text-fc-fg-muted">
								{#if sheetLabel(workbook)}{sheetLabel(workbook)} · {/if}{workbook.trashed_at ? `Trashed ${ago(workbook.trashed_at)}` : `Edited ${ago(workbook.updated_at)}`}
							</span>
						</span>
					</Card>
					<Menu
						label="Actions for {workbook.title}"
						items={actions(workbook)}
						align="end"
						class="absolute right-5 top-5 rounded-fc-md shadow-sm transition-opacity focus-within:opacity-100 group-hover/workbook:opacity-100 motion-reduce:transition-none [@media(hover:hover)_and_(pointer:fine)]:opacity-0"
					>
						{#snippet trigger()}<Icon icon={icons.more} size={18} />{/snippet}
					</Menu>
				</li>
			{/each}
			{#if showAddCard}
				<li class="h-full">
					<button
						type="button"
						onclick={create}
						disabled={busy}
						class="group flex h-full min-h-[220px] w-full flex-col items-center justify-center gap-3 rounded-fc-lg bg-fc-surface/50 p-4 text-center transition-[background-color,translate] duration-300 hover:-translate-y-1 hover:bg-fc-surface focus-visible:ring-2 focus-visible:ring-fc-accent motion-reduce:transition-none cursor-pointer disabled:cursor-not-allowed"
						aria-label="New workbook"
					>
						<span
							class="flex size-10 items-center justify-center rounded-full bg-fc-component text-fc-fg-muted transition-colors group-hover:bg-fc-page group-hover:text-fc-fg"
						>
							<Icon icon={icons.plus} size={20} />
						</span>
						<span class="flex flex-col gap-0.5">
							<span class="text-fc-sm font-medium text-fc-fg">New workbook</span>
							<span class="text-fc-xs text-fc-fg-muted">Create spreadsheet</span>
						</span>
					</button>
				</li>
			{/if}
		</ul>
	{/if}
</div>

<RenameModal bind:open={renameOpen} title={renaming?.title ?? ''} onSave={rename} />

<ContextMenu bind:this={contextMenu} label="Workbook actions" items={menuTarget ? actions(menuTarget) : []} returnFocus={() => menuReturn} />
