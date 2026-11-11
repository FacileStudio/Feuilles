import { backend, signInUrl, type SessionUser } from '$lib/backend';

/** The signed-in person. `loaded` turns true once `/api/auth/session` has answered. */
export const account = $state<{ user: SessionUser | null; loaded: boolean; error: string | null }>({
	user: null,
	loaded: false,
	error: null
});

/** Loads the session once per page load, then refreshes the Registre profile behind it. */
export async function loadAccount(): Promise<void> {
	try {
		account.user = await backend.session();
		account.error = null;
	} catch (error) {
		account.error = error instanceof Error ? error.message : String(error);
	} finally {
		account.loaded = true;
	}
	if (account.user) void refreshProfile();
}

/** Asks the API to re-read the Registre profile, and reloads the session when it changed. */
export async function refreshProfile(): Promise<void> {
	const synced = await backend.syncProfile().catch(() => false);
	if (synced) account.user = (await backend.session().catch(() => account.user)) ?? account.user;
}

/** Signs out, then goes back to the sign-in screen. */
export async function signOut(): Promise<void> {
	await backend.logout().catch(() => undefined);
	account.user = null;
	window.location.assign('/');
}

/** Sends the browser to Registre. */
export function signIn(): void {
	window.location.assign(signInUrl);
}
