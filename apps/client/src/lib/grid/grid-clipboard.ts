import { toast } from '@facile/muse';
import type { Session } from '$lib/engine/session.svelte';
import { copySelection, INTERNAL_MIME, pasteInto } from './clipboard';

/** Copy, cut and paste handlers for the grid element. They stand aside while a cell is being edited. */
export function clipboardHandlers(session: () => Session, editing: () => boolean) {
	function write(event: ClipboardEvent, type: 'copy' | 'cut'): boolean {
		const view = session().view;
		if (editing() || !view || !event.clipboardData) return false;
		const payload = copySelection(view, type);
		rememberCopy(payload);
		event.clipboardData.setData('text/plain', payload.text);
		event.clipboardData.setData(INTERNAL_MIME, payload.json);
		event.preventDefault();
		return true;
	}

	return {
		copy: (event: ClipboardEvent) => void write(event, 'copy'),
		cut: (event: ClipboardEvent) => {
			if (session().canEdit) write(event, 'cut');
		},
		paste: (event: ClipboardEvent) => {
			if (editing() || !event.clipboardData) return;
			event.preventDefault();
			if (!session().canEdit) return void toast.info('This workbook is read-only for you');
			const json = event.clipboardData.getData(INTERNAL_MIME) || null;
			const text = event.clipboardData.getData('text/plain');
			if (!json && !text) return;
			session().edit((model) => pasteInto(model, json, text));
		}
	};
}

let lastCopy: { text: string; json: string } | null = null;

/** Copies from the context menu by asking the browser to fire a copy event on the grid. */
export function menuCopy(grid: HTMLElement | null, cut = false): void {
	grid?.focus();
	document.execCommand(cut ? 'cut' : 'copy');
}

/** Remembers the last internal copy, so a menu paste of the same text keeps formulas and styles. */
export function rememberCopy(payload: { text: string; json: string }): void {
	lastCopy = payload;
}

/** Pastes from the context menu through the async Clipboard API, which carries plain text only. */
export async function menuPaste(session: Session): Promise<void> {
	const text = await navigator.clipboard.readText().catch(() => null);
	if (text === null) return void toast.info('Use Ctrl+V or Cmd+V to paste here');
	const json = lastCopy && lastCopy.text === text ? lastCopy.json : null;
	session.edit((model) => pasteInto(model, json, text));
}
