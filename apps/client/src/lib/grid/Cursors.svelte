<script lang="ts">
	import type { Model } from '@ironcalc/wasm';
	import type { PresenceEntry } from '$lib/engine/protocol';
	import { getColor } from './canvas/palette';
	import { headerColumnWidth, headerRowHeight } from './canvas/vendor/components/WorksheetCanvas/constants';
	import type WorksheetCanvas from './canvas/vendor/components/WorksheetCanvas/worksheetCanvas';

	let {
		canvas,
		view,
		presence,
		revision
	}: { canvas: WorksheetCanvas; view: Model; presence: PresenceEntry[]; revision: number } = $props();

	const boxes = $derived.by(() => {
		void revision;
		const sheet = view.getSelectedSheet();
		return presence.flatMap((entry) => {
			const cursor = entry.cursor;
			if (!cursor || cursor.sheet !== sheet) return [];
			const [r1, c1, r2, c2] = cursor.range;
			const [x, y] = canvas.getCoordinatesByCell(Math.min(r1, r2), Math.min(c1, c2));
			const [x2, y2] = canvas.getCoordinatesByCell(Math.max(r1, r2) + 1, Math.max(c1, c2) + 1);
			if (x2 <= headerColumnWidth || y2 <= headerRowHeight || x >= canvas.width || y >= canvas.height) return [];
			const left = Math.max(x, headerColumnWidth);
			const top = Math.max(y, headerRowHeight);
			return [{ id: entry.client_id, name: entry.name || 'Someone', color: getColor(entry.client_id), left, top, width: x2 - left, height: y2 - top }];
		});
	});
</script>

{#each boxes as box (box.id)}
	<div
		class="pointer-events-none absolute z-[5] rounded-[2px]"
		style:left="{box.left}px"
		style:top="{box.top}px"
		style:width="{box.width}px"
		style:height="{box.height}px"
		style:outline="2px solid {box.color}"
	>
		<span
			class="absolute -top-5 left-0 flex items-center gap-1 rounded-fc-sm bg-fc-component px-1.5 text-[11px] leading-5 font-medium whitespace-nowrap text-fc-fg"
		>
			<span class="size-2 rounded-full" style:background-color={box.color}></span>
			{box.name}
		</span>
	</div>
{/each}
