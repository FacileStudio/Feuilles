import { SQL, type Subprocess } from 'bun';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

/** What a running stack hands the tests: the app's URL and one session cookie per person. */
export type Stack = { base: string; space: string; cookies: Record<'alice' | 'bob' | 'outsider', string>; stop: () => Promise<void> };

const root = resolve(import.meta.dir, '../../..');
const api = resolve(root, 'apps/api/target/debug/api');
const seeder = resolve(root, 'apps/api/target/debug/examples/e2e_seed');
const clientDir = resolve(root, 'apps/client/build');
const admin = process.env.E2E_ADMIN_URL ?? 'postgres://postgres:postgres@127.0.0.1:5653/postgres?sslmode=disable';
const database = 'feuilles_e2e';

function freePort(): number {
	const probe = Bun.serve({ port: 0, fetch: () => new Response() });
	const port = probe.port ?? 0;
	probe.stop(true);
	return port;
}

/** An OIDC discovery document and an empty key set: enough for porte to boot, never a login. */
function fakeIssuer(): { url: string; stop: () => void } {
	const server = Bun.serve({
		port: 0,
		hostname: '127.0.0.1',
		fetch(request) {
			const url = `http://127.0.0.1:${server.port}`;
			const path = new URL(request.url).pathname;
			if (path === '/jwks') return Response.json({ keys: [] });
			if (path !== '/.well-known/openid-configuration') return new Response('not here', { status: 404 });
			return Response.json({
				issuer: url,
				authorization_endpoint: `${url}/authorize`,
				token_endpoint: `${url}/token`,
				jwks_uri: `${url}/jwks`,
				response_types_supported: ['code'],
				subject_types_supported: ['public'],
				id_token_signing_alg_values_supported: ['RS256']
			});
		}
	});
	return { url: `http://127.0.0.1:${server.port}`, stop: () => server.stop(true) };
}

async function freshDatabase(): Promise<string> {
	const sql = new SQL(admin);
	await sql.unsafe(`DROP DATABASE IF EXISTS ${database} WITH (FORCE)`);
	await sql.unsafe(`CREATE DATABASE ${database}`);
	await sql.close();
	const url = new URL(admin);
	url.pathname = `/${database}`;
	return url.toString();
}

const readySeconds = Number(process.env.E2E_READY_SECONDS ?? 120);

async function ready(base: string, server: Subprocess): Promise<void> {
	for (let attempt = 0; attempt < readySeconds * 10; attempt += 1) {
		if (server.exitCode !== null) throw new Error(`the API exited with ${server.exitCode}`);
		const answer = await fetch(`${base}/ready`).catch(() => null);
		if (answer?.ok) return;
		await Bun.sleep(100);
	}
	throw new Error(`the API did not become ready in ${readySeconds} seconds`);
}

async function seed(env: Record<string, string>): Promise<{ space: string; cookies: Stack['cookies'] }> {
	const run = Bun.spawn([seeder], { env: { ...process.env, ...env }, stdout: 'pipe', stderr: 'inherit' });
	const out = await new Response(run.stdout).text();
	if ((await run.exited) !== 0) throw new Error('e2e_seed failed');
	return JSON.parse(out);
}

/** Starts Postgres-backed Feuilles on a free port with the built client, then seeds it. */
export async function startStack(): Promise<Stack> {
	for (const path of [api, seeder, `${clientDir}/index.html`]) {
		if (!existsSync(path)) throw new Error(`${path} is missing: run "mise run e2e", which builds it`);
	}
	const issuer = fakeIssuer();
	const port = freePort();
	const base = `http://127.0.0.1:${port}`;
	const env = {
		DATABASE_URL: await freshDatabase(),
		PORT: String(port),
		CLIENT_DIR: clientDir,
		OIDC_ISSUER: issuer.url,
		OIDC_CLIENT_ID: 'feuilles-e2e',
		OIDC_CLIENT_SECRET: 'e2e-only',
		OIDC_REDIRECT_URL: `${base}/api/auth/oidc/callback`,
		OIDC_SUCCESS_URL: `${base}/`,
		LOG_LEVEL: process.env.LOG_LEVEL ?? 'warn'
	};
	const server = Bun.spawn([api], { env: { ...process.env, ...env }, stdout: 'inherit', stderr: 'inherit' });
	await ready(base, server);
	const seeded = await seed(env);
	const stop = async () => {
		server.kill('SIGTERM');
		await server.exited;
		issuer.stop();
	};
	return { base, ...seeded, stop };
}
