/** A space the caller belongs to, from `GET /api/spaces`. */
export type Space = { id: string; name: string; role: 'member' | 'admin' | 'owner' };

/** One sheet of a workbook, as `docs/api.md` gives `SheetInfo`. */
export type SheetInfo = { index: number; name: string; used_range: string | null };

/** A workbook as `docs/api.md` serialises it. `engine_version` is read when the API sends it. */
export type Workbook = {
	id: string;
	space_id: string | null;
	title: string;
	locale: string;
	timezone: string;
	language: string;
	sheets: SheetInfo[];
	created_at: string;
	updated_at: string;
	trashed_at: string | null;
	engine_version?: string;
};

/** The signed-in person, from `GET /api/auth/session`. */
export type SessionUser = { id: number; email: string; name: string; avatar_url?: string };

/** `GET /version`. `engine` is the server's `ironcalc` version (plan 7.7). */
export type ServerVersion = { version: string; engine?: string };

/** The suite error codes of `tronc/errors`. */
export type ErrorCode =
	| 'invalid_argument'
	| 'unauthenticated'
	| 'permission_denied'
	| 'not_found'
	| 'already_exists'
	| 'failed_precondition'
	| 'resource_exhausted'
	| 'rate_limited'
	| 'method_not_allowed'
	| 'unavailable'
	| 'internal'
	| 'network';
