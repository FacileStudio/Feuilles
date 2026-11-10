<script lang="ts">
	import { ColorPicker, Icon, IconButton, Menu, Popover, USER_COLORS, type MenuEntry } from '@facile/muse';
	import type { Session } from '$lib/engine/session.svelte';
	import { icons } from '$lib/icons';
	import type { GridCommands, NumberFormat } from './commands';
	import type { Toggle } from './keyboard';

	let { session, commands }: { session: Session; commands: GridCommands } = $props();

	const style = $derived.by(() => {
		void session.revision;
		const view = session.view;
		if (!view) return null;
		const [sheet, row, column] = view.getSelectedCell();
		return view.getCellStyle(sheet, row, column).style;
	});
	const off = $derived(!session.canEdit);

	const toggles: { id: Toggle; label: string; icon: string }[] = [
		{ id: 'bold', label: 'Bold', icon: icons.bold },
		{ id: 'italic', label: 'Italic', icon: icons.italic },
		{ id: 'underline', label: 'Underline', icon: icons.underline }
	];
	const pressed = (id: Toggle) => Boolean(style?.font[id === 'bold' ? 'b' : id === 'italic' ? 'i' : 'u']);

	const aligns = [
		{ id: 'left', label: 'Align left', icon: icons.alignLeft },
		{ id: 'center', label: 'Align centre', icon: icons.alignCenter },
		{ id: 'right', label: 'Align right', icon: icons.alignRight }
	] as const;

	const formats: [NumberFormat, string][] = [
		['general', 'Automatic'],
		['number', 'Number 1,234.56'],
		['percent', 'Percent 12.34%'],
		['currency', 'Currency €1,234.56'],
		['date', 'Date 2026-11-10']
	];
	const formatItems = $derived<MenuEntry[]>(formats.map(([id, label]) => ({ label, disabled: off, onselect: () => commands.format(id) })));
	const lineItems = $derived<MenuEntry[]>([
		{ label: 'Insert row above', disabled: off, onselect: () => commands.insertRows(false) },
		{ label: 'Insert row below', disabled: off, onselect: () => commands.insertRows(true) },
		{ label: 'Insert column left', disabled: off, onselect: () => commands.insertColumns(false) },
		{ label: 'Insert column right', disabled: off, onselect: () => commands.insertColumns(true) },
		{ separator: true },
		{ label: 'Delete rows', tone: 'danger', disabled: off, onselect: () => commands.deleteLines(false) },
		{ label: 'Delete columns', tone: 'danger', disabled: off, onselect: () => commands.deleteLines(true) }
	]);
</script>

{#snippet colour(label: string, icon: string, path: string)}
	<Popover {label}>
		{#snippet trigger()}<Icon {icon} size={18} />{/snippet}
		<div class="flex w-60 flex-col gap-3 p-3">
			<ColorPicker swatches={USER_COLORS} disabled={off} onChange={(hex) => commands.style(path, hex)} />
			<button type="button" class="self-start text-fc-sm text-fc-fg-muted hover:text-fc-fg" disabled={off} onclick={() => commands.style(path, '')}>
				Reset
			</button>
		</div>
	</Popover>
{/snippet}

<div class="flex shrink-0 items-center gap-1 overflow-x-auto bg-fc-component px-2 py-1" role="toolbar" aria-label="Formatting">
	<IconButton variant="ghost" aria-label="Undo" title="Undo" disabled={off} onclick={() => commands.history(false)}>
		<Icon icon={icons.undo} />
	</IconButton>
	<IconButton variant="ghost" aria-label="Redo" title="Redo" disabled={off} onclick={() => commands.history(true)}>
		<Icon icon={icons.redo} />
	</IconButton>
	<span class="mx-1 h-6 w-2 shrink-0" aria-hidden="true"></span>
	{#each toggles as toggle (toggle.id)}
		<IconButton
			variant={pressed(toggle.id) ? 'secondary' : 'ghost'}
			aria-label={toggle.label}
			aria-pressed={pressed(toggle.id)}
			title={toggle.label}
			disabled={off}
			onclick={() => commands.toggle(toggle.id)}
		>
			<Icon icon={toggle.icon} />
		</IconButton>
	{/each}
	{@render colour('Text colour', icons.text, 'font.color')}
	{@render colour('Fill colour', icons.fill, 'fill.fg_color')}
	<span class="mx-1 h-6 w-2 shrink-0" aria-hidden="true"></span>
	{#each aligns as align (align.id)}
		<IconButton
			variant={style?.alignment?.horizontal === align.id ? 'secondary' : 'ghost'}
			aria-label={align.label}
			title={align.label}
			disabled={off}
			onclick={() => commands.align(align.id)}
		>
			<Icon icon={align.icon} />
		</IconButton>
	{/each}
	<span class="mx-1 h-6 w-2 shrink-0" aria-hidden="true"></span>
	<Menu label="Number format" items={formatItems}>
		{#snippet trigger()}<Icon icon={icons.number} size={18} />{/snippet}
	</Menu>
	<Menu label="Rows and columns" items={lineItems}>
		{#snippet trigger()}<Icon icon={icons.layers} size={18} />{/snippet}
	</Menu>
	<IconButton variant="ghost" aria-label="Freeze panes at the active cell" title="Freeze panes" disabled={off} onclick={() => commands.freeze()}>
		<Icon icon={icons.freeze} />
	</IconButton>
</div>
