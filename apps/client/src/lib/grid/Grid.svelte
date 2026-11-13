<script lang="ts">
	import { onMount } from 'svelte';
	import type { Session } from '$lib/engine/session.svelte';
	import WorksheetCanvas from './canvas/vendor/components/WorksheetCanvas/worksheetCanvas';
	import { readThemeFromCSS } from './canvas/vendor/components/WorksheetCanvas/util';
	import { LAST_COLUMN, LAST_ROW } from './canvas/vendor/components/WorksheetCanvas/constants';
	import CellEditor from './CellEditor.svelte';
	import Cursors from './Cursors.svelte';
	import type { GridCommands } from './commands';
	import type { CellEdit } from './editing.svelte';
	import { GridPointer, type Zone } from './pointer';
	import { gridKeydown } from './grid-keys';
	import { clipboardHandlers } from './grid-clipboard';
	import './grid.css';

	let {
		session,
		commands,
		edit,
		onMenu
	}: {
		session: Session;
		commands: GridCommands;
		edit: CellEdit;
		onMenu: (x: number, y: number, zone: Zone) => void;
	} = $props();

	let root: HTMLDivElement;
	let scroller: HTMLDivElement;
	let spacer: HTMLDivElement;
	let sheet: HTMLDivElement;
	let canvasEl: HTMLCanvasElement;
	const parts: Record<string, HTMLDivElement> = {};
	let canvas = $state.raw<WorksheetCanvas | null>(null);
	let size = $state({ width: 0, height: 0 });
	let ignoreScroll = false;
	let synced: { x: number; y: number } | null = null;

	const redraw = () => session.touch();
	const pointer = new GridPointer({
		canvas: () => canvas,
		view: () => session.view,
		element: () => sheet,
		beforeSelect: () => edit.target && edit.commit(commands),
		changed: redraw
	});
	const clipboard = clipboardHandlers(() => session, () => edit.target !== null);

	function build(): WorksheetCanvas | null {
		const view = session.view;
		if (!view || !canvasEl) return null;
		return new WorksheetCanvas({
			model: view,
			width: sheet.clientWidth,
			height: sheet.clientHeight,
			workbookState: edit.state,
			elements: {
				canvas: canvasEl,
				cellOutline: parts.cellOutline,
				areaOutline: parts.areaOutline,
				cellArrayStructure: parts.arrayStructure,
				extendToOutline: parts.extendTo,
				columnGuide: parts.columnGuide,
				rowGuide: parts.rowGuide,
				columnHeaders: parts.columnHeaders,
				editor: parts.editor
			},
			onColumnWidthChanges: (s, column, width) => commands.resize(s, column, column, width, true),
			onRowHeightChanges: (s, row, height) => commands.resize(s, row, row, height, false),
			linkTooltipCell: null,
			refresh: () => commands.flush()
		});
	}

	function syncScroll(target: WorksheetCanvas): void {
		const x = target.model.getScrollX();
		const y = target.model.getScrollY();
		spacer.style.width = `${x + 100_000}px`;
		spacer.style.height = `${y + 500_000}px`;
		if (synced && synced.x === x && synced.y === y) return;
		const before = [scroller.scrollLeft, scroller.scrollTop];
		scroller.scrollLeft = x;
		scroller.scrollTop = y;
		ignoreScroll = scroller.scrollLeft !== before[0] || scroller.scrollTop !== before[1];
		synced = { x, y };
	}

	function onScroll(): void {
		if (ignoreScroll || !canvas) {
			ignoreScroll = false;
			return;
		}
		canvas.setScrollPosition({ left: scroller.scrollLeft, top: scroller.scrollTop });
		synced = { x: canvas.model.getScrollX(), y: canvas.model.getScrollY() };
		redraw();
	}

	function sendCursor(): void {
		const view = session.view;
		if (!view) return;
		const { sheet: s, row, column, range } = view.getSelectedView();
		session.setCursor({ sheet: s, row, column, range });
	}

	$effect(() => {
		void session.revision;
		const view = session.view;
		canvas ??= build();
		if (!canvas || !view || size.width === 0) return;
		canvas.model = view;
		canvas.theme = readThemeFromCSS(root);
		view.setWindowWidth(size.width);
		view.setWindowHeight(size.height);
		canvas.setSize(size);
		syncScroll(canvas);
		canvas.renderSheet();
		sendCursor();
	});

	onMount(() => {
		const observer = new ResizeObserver(() => (size = { width: sheet.clientWidth, height: sheet.clientHeight }));
		observer.observe(sheet);
		const theme = new MutationObserver(redraw);
		theme.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme', 'style'] });
		const scheme = matchMedia('(prefers-color-scheme: dark)');
		scheme.addEventListener('change', redraw);
		return () => {
			observer.disconnect();
			theme.disconnect();
			scheme.removeEventListener('change', redraw);
		};
	});

	function contextMenu(event: MouseEvent): void {
		event.preventDefault();
		const hit = pointer.cellAt(event.clientX, event.clientY);
		if (!hit || !session.view) return;
		const { range } = session.view.getSelectedView();
		const inside = hit.cell.row >= range[0] && hit.cell.row <= range[2] && hit.cell.column >= range[1] && hit.cell.column <= range[3];
		if (!inside && hit.zone === 'rows') session.view.setSelectedRange(hit.cell.row, 1, hit.cell.row, LAST_COLUMN);
		else if (!inside && hit.zone === 'columns') session.view.setSelectedRange(1, hit.cell.column, LAST_ROW, hit.cell.column);
		else if (!inside) session.view.setSelectedCell(hit.cell.row, hit.cell.column);
		redraw();
		onMenu(event.clientX, event.clientY, hit.zone);
	}

	function doubleClick(): void {
		if (session.canEdit && session.view) edit.begin(session.view, 'cell');
	}
</script>

<div
	bind:this={root}
	class="ic-root feuilles-grid relative min-h-0 flex-1 outline-none"
	role="grid"
	aria-label="Sheet"
	aria-readonly={!session.canEdit}
	tabindex="0"
	onkeydown={(event) => gridKeydown(event, { session, commands, edit })}
	oncopy={clipboard.copy}
	oncut={clipboard.cut}
	onpaste={clipboard.paste}
>
	<div bind:this={scroller} class="ic-worksheet-wrapper" onscroll={onScroll}>
		<div bind:this={spacer} class="ic-worksheet-spacer"></div>
		<div
			bind:this={sheet}
			class="ic-worksheet-sheet-container"
			class:readonly={!session.canEdit}
			role="presentation"
			onpointerdown={(event) => pointer.down(event)}
			onpointermove={(event) => pointer.move(event)}
			onpointerup={(event) => pointer.up(event)}
			oncontextmenu={contextMenu}
			ondblclick={doubleClick}
		>
			<canvas bind:this={canvasEl} class="ic-worksheet-sheet-canvas"></canvas>
			<div bind:this={parts.arrayStructure} class="ic-worksheet-cell-array-structure"></div>
			<div bind:this={parts.cellOutline} class="ic-worksheet-cell-outline"></div>
			<div bind:this={parts.editor} class="ic-worksheet-editor-wrapper" class:hidden-editor={!edit.target}>
				<CellEditor {edit} {commands} focusRoot={() => root.focus()} />
			</div>
			<div bind:this={parts.areaOutline} class="ic-worksheet-area-outline"></div>
			<div bind:this={parts.extendTo} class="ic-worksheet-extend-to-outline"></div>
			<div bind:this={parts.columnGuide} class="ic-worksheet-column-resize-guide"></div>
			<div bind:this={parts.rowGuide} class="ic-worksheet-row-resize-guide"></div>
			<div bind:this={parts.columnHeaders} class="ic-worksheet-column-headers"></div>
			{#if canvas && session.view}
				<Cursors {canvas} view={session.view} presence={session.presence} revision={session.revision} />
			{/if}
		</div>
	</div>
</div>
