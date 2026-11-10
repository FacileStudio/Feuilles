<script lang="ts">
	import { Button, Field, Input, Modal } from '@facile/muse';
	import { cleanTitle } from './format';

	let {
		open = $bindable(false),
		title,
		onSave
	}: { open?: boolean; title: string; onSave: (title: string) => Promise<void> } = $props();

	let draft = $state('');
	let saving = $state(false);
	const valid = $derived(cleanTitle(draft) !== null);

	$effect(() => {
		if (open) draft = title;
	});

	async function save(event: SubmitEvent) {
		event.preventDefault();
		const clean = cleanTitle(draft);
		if (!clean) return;
		saving = true;
		await onSave(clean).finally(() => (saving = false));
		open = false;
	}
</script>

<Modal bind:open title="Rename workbook" onClose={() => (open = false)}>
	<form id="rename-workbook" class="flex flex-col gap-4" onsubmit={save}>
		<Field label="Title" helper="1 to 200 characters">
			{#snippet children({ id, describedBy })}
				<Input {id} aria-describedby={describedBy} bind:value={draft} maxlength={200} autocomplete="off" />
			{/snippet}
		</Field>
	</form>
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button type="submit" form="rename-workbook" disabled={!valid || saving}>Save</Button>
	{/snippet}
</Modal>
