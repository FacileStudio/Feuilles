import { backend, signInUrl, type SessionUser } from '$lib/backend';
import { goto } from '$app/navigation';
import { DEFAULT_BULLE } from '$lib/bulle';
import { rememberReturn, takeReturn } from './return';

/**
 * The signed-in person and the origin of their Bulle sidebar, from the API's `BULLE_URL`.
 * `loaded` turns true once `/api/auth/session` has answered.
 */
export const account = $state<{ user: SessionUser | null; bulle: string; loaded: boolean; error: string | null }>({
	user: null,
	bulle: DEFAULT_BULLE,
	loaded: false,
	error: null
});

/**
 * Loads the session once per page load, then refreshes the Registre profile behind it. A signed-in
 * load goes back to the page {@link signIn} was called from.
 */
export async function loadAccount(): Promise<void> {
	try {
		const answer = await backend.session();
		account.user = answer.user;
		account.bulle = answer.bulle_url || DEFAULT_BULLE;
		account.error = null;
	} catch (error) {
		account.error = error instanceof Error ? error.message : String(error);
	} finally {
		account.loaded = true;
	}
	if (!account.user) return;
	void refreshProfile();
	const back = takeReturn();
	if (back && back !== window.location.pathname + window.location.search) void goto(back);
}

/** Asks the API to re-read the Registre profile, and reloads the session when it changed. */
export async function refreshProfile(): Promise<void> {
	const synced = await backend.syncProfile().catch(() => false);
	if (!synced) return;
	const answer = await backend.session().catch(() => null);
	account.user = answer?.user ?? account.user;
}

/** Signs out, then goes back to the sign-in screen. */
export async function signOut(): Promise<void> {
	await backend.logout().catch(() => undefined);
	account.user = null;
	window.location.assign('/');
}

/** Sends the browser to Registre, then back to `returnTo` once signed in. */
export function signIn(returnTo?: string): void {
	if (returnTo) rememberReturn(returnTo);
	window.location.assign(signInUrl);
}
