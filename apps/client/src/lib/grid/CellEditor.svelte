<script lang="ts">
	import type { GridCommands } from './commands';
	import type { CellEdit } from './editing.svelte';
	import { editorAction } from './keyboard';

	let { edit, commands, focusRoot }: { edit: CellEdit; commands: GridCommands; focusRoot: () => void } = $props();

	let field: HTMLTextAreaElement;

	$effect(() => {
		if (edit.target?.focus !== 'cell' || !field) return;
		field.focus();
		field.setSelectionRange(field.value.length, field.value.length);
	});

	function keydown(event: KeyboardEvent) {
		const action = editorAction(event);
		if (action.type === 'none' || event.isComposing) return;
		event.preventDefault();
		event.stopPropagation();
		if (action.type === 'newline') return insertBreak();
		if (action.type === 'cancel') edit.cancel();
		else edit.commit(commands, action.then);
		focusRoot();
	}

	function insertBreak() {
		const at = field.selectionStart;
		edit.set(`${edit.text.slice(0, at)}\n${edit.text.slice(field.selectionEnd)}`);
		queueMicrotask(() => field.setSelectionRange(at + 1, at + 1));
	}
</script>

<textarea
	bind:this={field}
	class="bare block h-full w-full resize-none overflow-hidden bg-transparent px-1 font-fc-sans text-[12px] leading-tight text-fc-fg outline-none"
	aria-label="Cell content"
	spellcheck="false"
	value={edit.text}
	tabindex={edit.target ? 0 : -1}
	oninput={(event) => edit.set(event.currentTarget.value)}
	onkeydown={keydown}
	onpointerdown={(event) => event.stopPropagation()}
></textarea>
