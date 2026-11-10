import { icons as museIcons, registerIcons } from '@facile/muse';
import data from './icons-data.json';

registerIcons(data);

/**
 * Feuilles' icon vocabulary: muse's names, plus the Solar glyphs muse does not carry, bundled in
 * `icons-data.json` the way muse bundles its own (Solar by 480 Design, CC BY 4.0, taken from
 * `@iconify-json/solar` 1.2.14). Importing this module registers them.
 */
export const icons = {
	...museIcons,
	bold: 'solar:text-bold-linear',
	italic: 'solar:text-italic-linear',
	underline: 'solar:text-underline-linear',
	undo: 'solar:undo-left-linear',
	redo: 'solar:undo-right-linear',
	alignLeft: 'solar:align-left-linear',
	alignCenter: 'solar:align-horizontal-center-linear',
	alignRight: 'solar:align-right-linear',
	fill: 'solar:paint-roller-linear',
	currency: 'solar:dollar-linear',
	number: 'solar:hashtag-linear',
	freeze: 'solar:snowflake-linear',
	workbook: 'solar:table-linear',
	paste: 'solar:clipboard-linear'
} as const;
