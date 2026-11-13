/** Whether the page runs inside another app's iframe, as Docs embeds a workbook with `?embed=1`. */
export const embedded = (url: URL) => url.searchParams.get('embed') === '1';
