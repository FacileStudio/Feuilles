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
	/** Null while the server's boot backfill has not counted an older workbook. */
	sheet_count?: number | null;
	created_at: string;
	updated_at: string;
	trashed_at: string | null;
	engine_version?: string;
};

/** The signed-in person, from `GET /api/auth/session`. */
export type SessionUser = { id: number; email: string; name: string; avatar_url?: string };

/** `GET /api/auth/session`: the account, or null, and the origin of the Bulle sidebar. */
export type SessionAnswer = { user: SessionUser | null; bulle_url?: string };

/** One agent action waiting for the person, from `GET /api/mcp/actions`. */
export interface PendingAction {
	id: string;
	user_id: string;
	kind: string;
	target: string;
	summary: string;
	client_id: string;
	created_at: string;
	expires_at: string;
	state: 'ready' | 'changed' | 'missing';
}

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
