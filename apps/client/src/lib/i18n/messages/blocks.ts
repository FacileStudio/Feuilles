import type { Row } from '../row';

/** Copy for the suite connections, the plan note and the developer block. */
export const blocks = {
	'landing.with.line': [
		'Your sheets travel to the other Facile apps.',
		'Vos tableaux passent dans les autres apps Facile.',
		'Tus hojas viajan a las demás apps de Facile.',
		'Ihre Tabellen wandern in die anderen Facile-Apps.'
	],
	'landing.with.docs': [
		'Your sheets live inside your Docs pages.',
		'Vos tableaux vivent dans vos pages Docs.',
		'Tus hojas viven dentro de tus páginas de Docs.',
		'Ihre Tabellen leben in Ihren Docs-Seiten.'
	],
	'landing.with.toile': [
		'A Feuilles link becomes a card on the board.',
		'Un lien Feuilles devient une carte sur la toile.',
		'Un enlace de Feuilles se convierte en tarjeta en la pizarra.',
		'Ein Feuilles-Link wird zur Karte auf dem Board.'
	],
	'landing.with.bulle': [
		'Fills in or sums up a sheet for you.',
		'Remplit ou résume une feuille pour vous.',
		'Rellena o resume una hoja por ti.',
		'Füllt eine Tabelle aus oder fasst sie zusammen.'
	],
	'landing.plan.included': [
		'Feuilles is included, the free plan too.',
		'Feuilles est inclus, offre gratuite comprise.',
		'Feuilles está incluido, también en el plan gratuito.',
		'Feuilles ist enthalten, auch im kostenlosen Tarif.'
	],
	'landing.plan.paid': ['Feuilles is included.', 'Feuilles est inclus.', 'Feuilles está incluido.', 'Feuilles ist enthalten.'],
	'landing.dev.line': [
		'Your agents read and edit your workbooks. Open source, and you can run it yourself.',
		'Vos agents lisent et modifient vos classeurs. Le code est ouvert, vous pouvez l\'installer chez vous.',
		'Tus agentes leen y editan tus libros. Código abierto, y puedes instalarlo tú mismo.',
		'Ihre Agenten lesen und bearbeiten Ihre Arbeitsmappen. Open Source, auch selbst betreibbar.'
	],
	'landing.dev.mcp': [
		'Agents read, fill in and restructure workbooks. Deleting cells or sheets waits for your yes.',
		'Les agents lisent, remplissent et réorganisent vos classeurs. Supprimer des cellules ou des feuilles attend votre accord.',
		'Los agentes leen, rellenan y reorganizan tus libros. Borrar celdas u hojas espera tu sí.',
		'Agenten lesen, füllen und ordnen Arbeitsmappen neu. Zellen oder Blätter löschen wartet auf Ihr Ja.'
	],
	'landing.dev.api': [
		'Every route documented, OpenAPI included.',
		'Chaque route est documentée, OpenAPI compris.',
		'Cada ruta documentada, OpenAPI incluido.',
		'Jede Route dokumentiert, OpenAPI inklusive.'
	],
	'landing.dev.source': [
		'AGPL, built on the IronCalc engine.',
		'Sous licence AGPL, sur le moteur IronCalc.',
		'AGPL, sobre el motor IronCalc.',
		'AGPL, auf der IronCalc-Engine.'
	]
} as const satisfies Record<string, Row>;
