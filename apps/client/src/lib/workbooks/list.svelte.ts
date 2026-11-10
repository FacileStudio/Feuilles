import { toast } from '@facile/muse';
import { ApiError, backend, type Workbook } from '$lib/backend';

/** Why a call failed, in words a person can act on. */
export function explain(error: unknown): string {
	if (error instanceof ApiError) return error.message;
	return error instanceof Error ? error.message : 'Something went wrong';
}

/** The workbooks of one scope: the live list, or the trash. */
export class WorkbookList {
	items = $state<Workbook[]>([]);
	loading = $state(true);
	error = $state<string | null>(null);

	constructor(private readonly trashed: boolean) {}

	/** Loads the list for a space, or the personal scope when `spaceId` is null. */
	async load(spaceId: string | null): Promise<void> {
		this.loading = true;
		try {
			this.items = await backend.list(spaceId, this.trashed);
			this.error = null;
		} catch (error) {
			this.error = explain(error);
		} finally {
			this.loading = false;
		}
	}

	/** Runs a call that changes one workbook, then drops it from this list or replaces it. */
	async act(id: string, call: () => Promise<Workbook | void>, done: string): Promise<void> {
		try {
			const next = await call();
			const stays = next ? (next.trashed_at !== null) === this.trashed : false;
			this.items = stays
				? this.items.map((item) => (item.id === id && next ? next : item))
				: this.items.filter((item) => item.id !== id);
			toast.success(done);
		} catch (error) {
			toast.danger(explain(error));
		}
	}
}
