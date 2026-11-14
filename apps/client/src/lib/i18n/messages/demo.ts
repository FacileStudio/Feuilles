import type { Row } from '../row';

/** Words shown inside the hero sheet and the three demo tiles. */
export const demo = {
	'demo.label': [
		'A Feuilles workbook edited by three people at once: Léa fills in the costs, you write a SUM that updates as they land, and Malik formats the header row.',
		'Un classeur Feuilles modifié par trois personnes à la fois : Léa saisit les coûts, vous écrivez une SUM qui se met à jour à chaque saisie, et Malik met en forme la ligne d\'en-tête.',
		'Un libro de Feuilles editado por tres personas a la vez: Léa rellena los costes, tú escribes una SUM que se actualiza con cada dato y Malik da formato a la fila de cabecera.',
		'Eine Feuilles-Arbeitsmappe, die drei Personen gleichzeitig bearbeiten: Léa trägt die Kosten ein, Sie schreiben eine SUM, die mitrechnet, und Malik formatiert die Kopfzeile.'
	],
	'demo.back': ['Workbooks', 'Classeurs', 'Libros', 'Arbeitsmappen'],
	'demo.book': ['Q4 budget', 'Budget T4', 'Presupuesto T4', 'Q4-Budget'],
	'demo.tab': ['Budget', 'Budget', 'Presupuesto', 'Budget'],
	'demo.head.item': ['Item', 'Poste', 'Concepto', 'Posten'],
	'demo.head.cost': ['Cost', 'Coût', 'Coste', 'Kosten'],
	'demo.head.notes': ['Notes', 'Notes', 'Notas', 'Notizen'],
	'demo.row.hosting': ['Hosting', 'Hébergement', 'Alojamiento', 'Hosting'],
	'demo.row.design': ['Design', 'Design', 'Diseño', 'Design'],
	'demo.row.ads': ['Ads', 'Pub', 'Anuncios', 'Werbung'],
	'demo.row.tools': ['Tools', 'Outils', 'Herramientas', 'Tools'],
	'demo.row.rent': ['Rent', 'Loyer', 'Alquiler', 'Miete'],
	'demo.row.sales': ['Sales', 'Ventes', 'Ventas', 'Umsatz'],
	'demo.row.costs': ['Costs', 'Coûts', 'Costes', 'Kosten'],
	'demo.total': ['Total', 'Total', 'Total', 'Summe'],
	'demo.import.title': ['Open your Excel file', 'Ouvrez votre fichier Excel', 'Abre tu archivo de Excel', 'Öffnen Sie Ihre Excel-Datei'],
	'demo.import.body': [
		'Drop an Excel or CSV file and keep working.',
		'Déposez un fichier Excel ou CSV et continuez votre travail.',
		'Suelta un archivo de Excel o CSV y sigue trabajando.',
		'Legen Sie eine Excel- oder CSV-Datei ab und arbeiten Sie weiter.'
	],
	'demo.import.drop': ['Import an Excel file', 'Importer un fichier Excel', 'Importar un Excel', 'Excel-Datei importieren'],
	'demo.import.book': ['Budget 2026', 'Budget 2026', 'Presupuesto 2026', 'Budget 2026'],
	'demo.import.meta': ['3 sheets · Just now', '3 feuilles · À l\'instant', '3 hojas · Ahora mismo', '3 Blätter · Gerade eben'],
	'demo.embed.title': ['Inside your docs', 'Dans vos pages Docs', 'Dentro de tus Docs', 'In Ihren Docs'],
	'demo.embed.body': [
		'A live sheet inside your Docs page.',
		'Un tableau vivant dans votre page Docs.',
		'Una hoja en vivo dentro de tu página de Docs.',
		'Eine Live-Tabelle in Ihrer Docs-Seite.'
	],
	'demo.embed.page': ['Q4 review', 'Bilan du T4', 'Revisión del T4', 'Q4-Rückblick'],
	'demo.bulle.title': ['Ask Bulle', 'Demandez à Bulle', 'Pregunta a Bulle', 'Fragen Sie Bulle'],
	'demo.bulle.body': [
		'Bulle fills in or sums up a sheet for you.',
		'Bulle remplit ou résume une feuille pour vous.',
		'Bulle rellena o resume una hoja por ti.',
		'Bulle füllt eine Tabelle aus oder fasst sie zusammen.'
	],
	'demo.bulle.ask': ['Add up the costs', 'Additionnez les coûts', 'Suma los costes', 'Bitte die Kosten addieren'],
	'demo.bulle.reply': [
		'Done, the total is in B4.',
		'C\'est fait, le total est en B4.',
		'Hecho, el total está en B4.',
		'Erledigt, die Summe steht in B4.'
	]
} as const satisfies Record<string, Row>;
