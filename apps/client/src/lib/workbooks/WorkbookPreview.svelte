<script lang="ts">
	import type { Workbook } from '$lib/backend';

	let { workbook }: { workbook: Workbook } = $props();

	const sheetCount = $derived(
		workbook.sheets.length === 1 ? '1 sheet' : `${workbook.sheets.length} sheets`
	);
</script>

<div
	class="relative flex aspect-[16/10] flex-col justify-between overflow-hidden rounded-fc-md bg-fc-surface p-3"
	aria-hidden="true"
>
	<div class="flex flex-col gap-1.5">
		<div class="flex items-center gap-1.5">
			<div class="h-3.5 w-5 rounded-fc-xs bg-fc-component"></div>
			<div class="h-3.5 flex-1 rounded-fc-xs bg-fc-component"></div>
			<div class="h-3.5 flex-1 rounded-fc-xs bg-fc-component"></div>
			<div class="h-3.5 flex-1 rounded-fc-xs bg-fc-component"></div>
			<div class="h-3.5 flex-1 rounded-fc-xs bg-fc-component"></div>
		</div>

		<div class="flex items-center gap-1.5">
			<div class="h-4 w-5 rounded-fc-xs bg-fc-component/50"></div>
			<div class="flex h-4 flex-1 items-center rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-3/4 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center justify-end rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-1/2 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-2/3 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center justify-end rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-3/5 rounded-full bg-fc-component"></div>
			</div>
		</div>

		<div class="flex items-center gap-1.5">
			<div class="h-4 w-5 rounded-fc-xs bg-fc-component/50"></div>
			<div class="flex h-4 flex-1 items-center rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-1/2 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center justify-end rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-2/3 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-3/4 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center justify-end rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-2/5 rounded-full bg-fc-component"></div>
			</div>
		</div>

		<div class="flex items-center gap-1.5">
			<div class="h-4 w-5 rounded-fc-xs bg-fc-component/50"></div>
			<div class="flex h-4 flex-1 items-center rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-2/3 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center justify-end rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-1/3 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-1/2 rounded-full bg-fc-component"></div>
			</div>
			<div class="flex h-4 flex-1 items-center justify-end rounded-fc-xs bg-fc-page/50 px-1.5">
				<div class="h-1.5 w-3/4 rounded-full bg-fc-component"></div>
			</div>
		</div>
	</div>

	<div class="flex items-center justify-between pt-1">
		<div class="flex items-center gap-1.5 rounded-fc-sm bg-fc-component px-2 py-0.5">
			<span class="size-1.5 rounded-full bg-emerald-500"></span>
			<span class="max-w-[100px] truncate text-fc-xs font-medium text-fc-fg">
				{workbook.sheets[0]?.name || 'Sheet1'}
			</span>
		</div>
		<span class="rounded-fc-xs bg-fc-page/50 px-1.5 py-0.5 text-fc-xs text-fc-fg-muted">
			{sheetCount}
		</span>
	</div>
</div>
