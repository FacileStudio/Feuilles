<script lang="ts">
	import type { Session } from '$lib/engine/session.svelte';
	import { getCellAddress } from './canvas/vendor/components/util';
	import type { GridCommands } from './commands';
	import type { CellEdit } from './editing.svelte';
	import { editorAction } from './keyboard';

	let {
		session,
		commands,
		edit,
		focusGrid
	}: { session: Session; commands: GridCommands; edit: CellEdit; focusGrid: () => void } = $props();

	const selected = $derived.by(() => {
		void session.revision;
		const view = session.view;
		if (!view) return { address: '', content: '', value: '' };
		const { sheet, row, column, range } = view.getSelectedView();
		const area = { rowStart: range[0], columnStart: range[1], rowEnd: range[2], columnEnd: range[3] };
		const address = getCellAddress(area, { row, column });
		return { address, content: view.getCellContent(sheet, row, column), value: view.getFormattedCellValue(sheet, row, column) };
	});

	function focus() {
		if (!edit.target && session.view && session.canEdit) edit.begin(session.view, 'formula-bar');
	}

	function keydown(event: KeyboardEvent) {
		const action = editorAction(event);
		if (action.type === 'none' || action.type === 'newline' || event.isComposing) return;
		event.preventDefault();
		if (action.type === 'cancel') edit.cancel();
		else edit.commit(commands, action.then);
		focusGrid();
	}
</script>

<div class="flex h-10 shrink-0 items-center gap-3 bg-fc-component px-3">
	<span class="w-20 shrink-0 truncate font-fc-mono text-fc-sm text-fc-fg-muted" aria-label="Selected cell">{selected.address}</span>
	<output class="sr-only" aria-label="Value of the selected cell">{selected.value}</output>
	<input
		class="bare h-8 min-w-0 flex-1 rounded-fc-sm bg-transparent px-2 font-fc-mono text-fc-sm text-fc-fg outline-none focus-visible:bg-fc-bg"
		aria-label="Formula"
		spellcheck="false"
		autocomplete="off"
		readonly={!session.canEdit}
		value={edit.target ? edit.text : selected.content}
		onfocus={focus}
		oninput={(event) => edit.set(event.currentTarget.value)}
		onkeydown={keydown}
	/>
</div>
