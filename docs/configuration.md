# Feuilles — Configuration

Every environment variable the API reads (`apps/api/src/env.rs`, tronc `env::Core`, porte
`Config::from_vars`). A missing required variable, or a value that does not parse, exits 1 at
boot with a message naming it.

| Variable | Required | Default | What it does |
|---|---|---|---|
| `DATABASE_URL` | yes | | Postgres connection string |
| `POSTGRES_PASSWORD` | compose | required | The bundled Postgres password, also used in the compose `DATABASE_URL` default. Compose refuses to start without it; set it in Dokploy |
| `PORT` | no | `8080` (tronc's default; the image sets `4000`) | HTTP listen port, also read by `api healthcheck` |
| `APP_ENV` | no | `development` | `production` requires `OIDC_ISSUER` and `PORTE_TOKEN_KEY` |
| `LOG_LEVEL` | no | `info` | `debug`, `info`, `warn`, `error` |
| `TRUSTED_PROXIES` | production | private ranges | `private,cloudflare` behind Traefik and Cloudflare |
| `CORS_ALLOWED_ORIGINS` | no | | Same origin in production, so empty |
| `CLIENT_DIR` | no | `./client` | The built SPA. Not served when `index.html` is missing |
| `JOURNAL_URL`, `JOURNAL_TOKEN` | no | | Ship logs to Journal when both are set |
| `OIDC_ISSUER` | production | | `https://sso.facile.studio`. Turns on sign-in, the directory sync and deletions |
| `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` | with the issuer | | Registre client credentials, also used for the directory |
| `OIDC_REDIRECT_URL` | with the issuer | | `https://feuilles.facile.studio/api/auth/oidc/callback` |
| `OIDC_SUCCESS_URL` | with the issuer | | `https://feuilles.facile.studio/` |
| `OIDC_CLI_AUDIENCE` | no | | Mounts `POST /api/auth/oidc/device/exchange` |
| `OIDC_MACHINE_AUDIENCE`, `OIDC_SCOPE_PREFIX` | no | | Accept bearer JWTs on the API, gated on `<prefix>.read` and `.write` |
| `OIDC_CLAIMS_SCOPE` | no | | Request and cache a roles claim |
| `OIDC_APP_NAME`, `OIDC_LOGO_URL` | no | | Shown on the CLI code page |
| `PORTE_TOKEN_KEY` | production | | Seals stored OIDC tokens at rest (porte `TokenSealer` spec) |
| `MCP_RESOURCE` | no | | `https://feuilles.facile.studio/mcp`. Its presence mounts `/mcp`; needs `OIDC_ISSUER` and `PORTE_TOKEN_KEY` |
| `BULLE_URL` | no | `https://bulle.facile.studio` | The Bulle origin the client frames as its sidebar; the CSP's `frame-src` allows it |

`SSO_ONLY` is read by porte but forced to true: local password routes are never mounted. Feuilles
calls no AI itself: the Bulle sidebar is Bulle's own page in a frame. `MCP_STATE_KEY` is not
read: the key that signs MCP confirmation questions is derived from `PORTE_TOKEN_KEY`.

Test-only: `TEST_DATABASE_URL` points the database tests at a scratch Postgres; each test binary
migrates its own schema.
