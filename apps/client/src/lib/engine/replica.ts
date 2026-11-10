import type { SelectedView } from '@ironcalc/wasm';

/** The part of an IronCalc `Model` the collaboration algorithm touches. */
export interface EngineModel {
	applyExternalDiffs(diffs: Uint8Array): void;
	flushSendQueue(): Uint8Array;
	toBytes(): Uint8Array;
	getSelectedView(): SelectedView;
	setSelectedSheet(sheet: number): void;
	setSelectedCell(row: number, column: number): void;
	setSelectedRange(startRow: number, startColumn: number, endRow: number, endColumn: number): void;
	setTopLeftVisibleCell(row: number, column: number): void;
	free(): void;
}

/** A local batch not yet echoed back by the server. */
export type Pending = { clientSeq: number; batch: Uint8Array };

/** An accepted batch, as UPDATE carries it. */
export type Accepted = { seq: number; clientId: number; clientSeq: number; payload: Uint8Array };

/** What happened to an UPDATE: applied, already seen, or out of order (the caller must resync). */
export type UpdateResult = 'applied' | 'duplicate' | 'desync';

const sameBytes = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((v, i) => v === b[i]);

/**
 * The browser half of plan 7.1. `confirmed` applies only server batches in `seq` order; `view`
 * is `confirmed` plus `pending`, and is rebuilt whenever a remote batch lands while a local one is
 * in flight. A pending batch that no longer replays on a rebuilt view is dropped: the server
 * refuses it too, and an UPDATE for it, if one comes, is then handled like a remote batch.
 */
export class Replica<M extends EngineModel> {
	confirmed: M;
	view: M;
	pending: Pending[] = [];
	seq: number;
	clientId: number;
	private nextClientSeq = 1;
	private readonly empty: Uint8Array;

	constructor(
		private readonly load: (bytes: Uint8Array) => M,
		snapshot: Uint8Array,
		seq: number,
		clientId: number
	) {
		this.confirmed = load(snapshot);
		this.view = load(snapshot);
		this.seq = seq;
		this.clientId = clientId;
		this.empty = this.confirmed.flushSendQueue();
	}

	/** Runs a local edit on `view`. Returns the batch to PUSH, or null when the edit produced none. */
	edit(change: (model: M) => void): Pending | null {
		let failed = false;
		try {
			change(this.view);
		} catch {
			failed = true;
		}
		const batch = this.view.flushSendQueue();
		const empty = sameBytes(batch, this.empty);
		const entry = empty ? null : { clientSeq: this.nextClientSeq++, batch };
		if (entry) this.pending.push(entry);
		if (failed) this.rebuild();
		return entry;
	}

	/** Throws away anything changed on `view` outside `edit`, for a session that may not edit. */
	discard(): void {
		if (!sameBytes(this.view.flushSendQueue(), this.empty)) this.rebuild();
	}

	/** Applies one UPDATE. */
	update(accepted: Accepted): UpdateResult {
		if (accepted.seq <= this.seq) return 'duplicate';
		if (accepted.seq !== this.seq + 1) return 'desync';
		try {
			this.confirmed.applyExternalDiffs(accepted.payload);
		} catch {
			return 'desync';
		}
		this.seq = accepted.seq;
		const mine = accepted.clientId === this.clientId ? this.takePending(accepted.clientSeq) : null;
		if (!mine) this.applyRemote(accepted.payload);
		return 'applied';
	}

	/** Handles REJECT: drops the batch and rebuilds `view` without it. */
	reject(clientSeq: number): void {
		this.takePending(clientSeq);
		this.rebuild();
	}

	/**
	 * Handles a fresh INIT after a reconnect: replaces `confirmed`, rebuilds `view` from the new
	 * snapshot and the pending batches, and renumbers what survived. Returns the batches to resend.
	 */
	reset(snapshot: Uint8Array, seq: number, clientId: number): Pending[] {
		this.confirmed.free();
		this.confirmed = this.load(snapshot);
		this.seq = seq;
		this.clientId = clientId;
		this.nextClientSeq = 1;
		this.rebuild();
		for (const entry of this.pending) entry.clientSeq = this.nextClientSeq++;
		return [...this.pending];
	}

	/** Rebuilds `view` as `confirmed` plus every pending batch that still replays, keeping the selection. */
	rebuild(): void {
		const selection = this.view.getSelectedView();
		const base = this.confirmed.toBytes();
		let failed = -1;
		do {
			this.view.free();
			this.view = this.load(base);
			if (failed >= 0) this.pending.splice(failed, 1);
			failed = this.pending.findIndex((entry) => !this.replays(entry));
		} while (failed >= 0);
		restoreSelection(this.view, selection);
	}

	/** Frees both WASM models. */
	dispose(): void {
		this.confirmed.free();
		this.view.free();
	}

	private replays(entry: Pending): boolean {
		try {
			this.view.applyExternalDiffs(entry.batch);
			return true;
		} catch {
			return false;
		}
	}

	private takePending(clientSeq: number): Pending | null {
		const index = this.pending.findIndex((entry) => entry.clientSeq === clientSeq);
		return index < 0 ? null : this.pending.splice(index, 1)[0];
	}

	private applyRemote(payload: Uint8Array): void {
		if (this.pending.length > 0) return this.rebuild();
		try {
			this.view.applyExternalDiffs(payload);
		} catch {
			this.rebuild();
		}
	}
}

/** Puts the selected sheet, cell, range and scroll position back after a rebuild. */
export function restoreSelection(model: EngineModel, selection: SelectedView): void {
	try {
		model.setSelectedSheet(selection.sheet);
		model.setSelectedCell(selection.row, selection.column);
		const [r1, c1, r2, c2] = selection.range;
		model.setSelectedRange(r1, c1, r2, c2);
		model.setTopLeftVisibleCell(selection.top_row, selection.left_column);
	} catch {
		model.setSelectedSheet(0);
	}
}
