import { describe, expect, it } from 'vitest';
import { editorAction, gridAction, type KeyPress } from './keyboard';

const press = (key: string, mods: Partial<KeyPress> = {}): KeyPress => ({
	key,
	shiftKey: false,
	ctrlKey: false,
	metaKey: false,
	altKey: false,
	...mods
});

describe('gridAction', () => {
	it('moves, expands and jumps with the arrows', () => {
		expect(gridAction(press('ArrowDown'))).toEqual({ type: 'move', arrow: 'ArrowDown' });
		expect(gridAction(press('ArrowLeft', { shiftKey: true }))).toEqual({ type: 'expand', arrow: 'ArrowLeft' });
		expect(gridAction(press('ArrowUp', { ctrlKey: true }))).toEqual({ type: 'edge', arrow: 'ArrowUp' });
		expect(gridAction(press('ArrowRight', { metaKey: true }))).toEqual({ type: 'edge', arrow: 'ArrowRight' });
	});

	it('moves sideways on Tab', () => {
		expect(gridAction(press('Tab'))).toEqual({ type: 'move', arrow: 'ArrowRight' });
		expect(gridAction(press('Tab', { shiftKey: true }))).toEqual({ type: 'move', arrow: 'ArrowLeft' });
	});

	it('starts editing on Enter and F2, and typing replaces the cell', () => {
		expect(gridAction(press('Enter'))).toEqual({ type: 'edit' });
		expect(gridAction(press('F2'))).toEqual({ type: 'edit' });
		expect(gridAction(press('='))).toEqual({ type: 'type', text: '=' });
		expect(gridAction(press('é'))).toEqual({ type: 'type', text: 'é' });
	});

	it('clears on Delete and Backspace, pages on PageUp and PageDown', () => {
		expect(gridAction(press('Delete'))).toEqual({ type: 'clear' });
		expect(gridAction(press('Backspace'))).toEqual({ type: 'clear' });
		expect(gridAction(press('PageDown'))).toEqual({ type: 'page', down: true });
	});

	it('maps the Ctrl and Cmd shortcuts', () => {
		expect(gridAction(press('z', { ctrlKey: true }))).toEqual({ type: 'undo' });
		expect(gridAction(press('Z', { metaKey: true, shiftKey: true }))).toEqual({ type: 'redo' });
		expect(gridAction(press('y', { ctrlKey: true }))).toEqual({ type: 'redo' });
		expect(gridAction(press('b', { ctrlKey: true }))).toEqual({ type: 'toggle', style: 'bold' });
		expect(gridAction(press('a', { metaKey: true }))).toEqual({ type: 'selectAll' });
	});

	it('leaves copy, paste and unknown keys to the browser', () => {
		expect(gridAction(press('c', { ctrlKey: true }))).toBeNull();
		expect(gridAction(press('v', { metaKey: true }))).toBeNull();
		expect(gridAction(press('Shift', { shiftKey: true }))).toBeNull();
		expect(gridAction(press('Escape'))).toBeNull();
		expect(gridAction(press('x', { altKey: true }))).toBeNull();
	});
});

describe('editorAction', () => {
	it('commits and moves on Enter and Tab', () => {
		expect(editorAction(press('Enter'))).toEqual({ type: 'commit', then: 'ArrowDown' });
		expect(editorAction(press('Enter', { shiftKey: true }))).toEqual({ type: 'commit', then: 'ArrowUp' });
		expect(editorAction(press('Tab'))).toEqual({ type: 'commit', then: 'ArrowRight' });
		expect(editorAction(press('Tab', { shiftKey: true }))).toEqual({ type: 'commit', then: 'ArrowLeft' });
	});

	it('cancels on Escape, breaks the line on Alt+Enter, ignores the rest', () => {
		expect(editorAction(press('Escape'))).toEqual({ type: 'cancel' });
		expect(editorAction(press('Enter', { altKey: true }))).toEqual({ type: 'newline' });
		expect(editorAction(press('a'))).toEqual({ type: 'none' });
	});
});
