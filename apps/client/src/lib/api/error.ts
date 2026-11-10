import type { ErrorCode } from './types';

const codeByStatus: Record<number, ErrorCode> = {
	400: 'invalid_argument',
	401: 'unauthenticated',
	403: 'permission_denied',
	404: 'not_found',
	405: 'method_not_allowed',
	409: 'already_exists',
	412: 'failed_precondition',
	413: 'resource_exhausted',
	429: 'rate_limited',
	503: 'unavailable'
};

/** A failed API call, typed from the suite envelope `{"error":{"code","message"}}`. */
export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: ErrorCode,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}

	/** Builds the error from a non-2xx response, falling back to the status when the body is not the envelope. */
	static async from(response: Response): Promise<ApiError> {
		const fallback = codeByStatus[response.status] ?? 'internal';
		const body = (await response.json().catch(() => null)) as { error?: { code?: string; message?: string } } | null;
		const code = (body?.error?.code as ErrorCode | undefined) ?? fallback;
		return new ApiError(response.status, code, body?.error?.message ?? `Request failed with status ${response.status}`);
	}
}
