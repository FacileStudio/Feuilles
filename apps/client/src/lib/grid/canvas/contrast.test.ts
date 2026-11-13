import { describe, expect, it } from 'vitest';
import { cellFill, cellInk, contrast, lineInk, luminance } from './contrast';

const light = { text: '#0a0a0a', background: '#ffffff' };
const dark = { text: '#fafafa', background: '#000000' };

describe('luminance and contrast', () => {
	it('reads short, long and alpha hex', () => {
		expect(luminance('#fff')).toBeCloseTo(1);
		expect(luminance('#000000')).toBe(0);
		expect(luminance('#FFFFFF80')).toBeCloseTo(1);
		expect(luminance('oklch(1 0 0)')).toBeNull();
	});

	it('gives 21:1 for black on white', () => {
		expect(contrast('#000', '#fff')).toBeCloseTo(21);
	});
});

describe('cellFill', () => {
	it('treats no fill and white fills as the theme background', () => {
		expect(cellFill(undefined)).toBeNull();
		expect(cellFill('#FFFFFF')).toBeNull();
		expect(cellFill('#F5F5F5')).toBeNull();
	});

	it('keeps real fills', () => {
		expect(cellFill('#FFF2CC')).toBe('#FFF2CC');
		expect(cellFill('#1F4E79')).toBe('#1F4E79');
	});
});

describe('cellInk', () => {
	it('maps automatic black text to the theme text in both themes', () => {
		expect(cellInk('#000000', light.background, light)).toBe(light.text);
		expect(cellInk('#000000', dark.background, dark)).toBe(dark.text);
		expect(cellInk(undefined, dark.background, dark)).toBe(dark.text);
	});

	it('keeps black text readable on a light fill in dark mode', () => {
		expect(cellInk('#000000', '#FFF2CC', dark)).toBe(dark.background);
	});

	it('keeps an explicit colour that reads', () => {
		expect(cellInk('#C00000', light.background, light)).toBe('#C00000');
		expect(cellInk('#FFFFFF', '#1F4E79', light)).toBe('#FFFFFF');
	});

	it('replaces an explicit colour under 3:1', () => {
		expect(cellInk('#FFFFFF', light.background, light)).toBe(light.text);
		expect(cellInk('#1F4E79', dark.background, dark)).toBe(dark.text);
		expect(cellInk('#333333', '#1F4E79', light)).toBe(light.background);
	});
});

describe('lineInk', () => {
	it('maps automatic borders to the theme text and keeps light greys', () => {
		expect(lineInk('#000000', dark.text)).toBe(dark.text);
		expect(lineInk('', dark.text)).toBe(dark.text);
		expect(lineInk('#D9D9D9', light.text)).toBe('#D9D9D9');
	});
});
