# Feuilles — Deployment

How the image is built, how it runs on la ruche, why there is exactly one replica, and how to
upgrade IronCalc without losing a workbook.

## Image

`Dockerfile` builds in three stages: the SvelteKit client with bun, the API with
`cargo build --release --locked` (dependencies cached in their own layer, the version stamped
from `apps/client/package.json` by `scripts/version.sh`, which fails the build when it cannot
read one), then `gcr.io/distroless/cc-debian12:nonroot` with `/api`, the client in `/client`,
`CLIENT_DIR=/client` and `PORT=4000`. TLS is rustls, so the image needs only libc.

The image is entrypoint-only:

```sh
docker run --rm <image> healthcheck
docker run --rm -e DATABASE_URL=... <image> migrate status
```

## Compose and Dokploy

`docker-compose.yml` follows Capsule's: `feuilles-db` (Postgres 16, volume `feuilles_db_data`)
and `feuilles-api` behind two Traefik routers on `feuilles.facile.studio`, port 4000, health
check `["CMD", "/api", "healthcheck"]`. One container, one router, one hostname: `/api/*` is the
API and everything else is the SPA, with no `PathPrefix` and no `stripprefix`.

Migrations run at boot behind an advisory lock (`tronc::migrate`). A failed migration exits 1.

## One replica

`feuilles-api` sets `deploy.replicas: 1` and `update_config.order: stop-first`. Collaboration
rooms are in memory: two replicas would each hold a model of the same workbook and order
batches differently. `stop-first` keeps a deploy from running the old and the new container side
by side. This holds until a pub/sub layer exists. A deploy disconnects every editor for a few
seconds; on SIGTERM each room writes its snapshot and closes its sockets with 1001, and clients
reconnect and resend.

The directory sync, the deletion poll and the hourly trash purge run in that one process.

## Upgrading IronCalc

Snapshots and batches are `bitcode` of IronCalc's private types. A new `ironcalc` may not read
old bytes, and the browser's `@ironcalc/wasm` must match the server exactly. Never bump
`ironcalc` in a routine dependency update. For a bump:

1. With the old image: `docker run --rm -e DATABASE_URL=... -v /srv/feuilles-backup:/backup <old-image> engine-export /backup`.
   It writes a fresh snapshot of every workbook and an `.xlsx` copy to `/backup/<id>.xlsx`.
2. Bump `ironcalc = "=X.Y.Z"` in `apps/api/Cargo.toml`, `ENGINE_VERSION` in
   `apps/api/src/engine/mod.rs` (a test checks they agree), and `@ironcalc/wasm` in the client.
3. Deploy the new image, and run step 4 before anyone opens a workbook.
4. With the new image: `engine-verify /backup`. It loads every workbook; one that fails is
   re-imported from its `.xlsx` copy. It stamps `engine_version` and exits non-zero when any
   workbook has neither.

## Before the first deploy

The crates are referenced by local `file://` git URLs while nothing is pushed, so the image
cannot build on a server. Once the owner pushes the libraries, edit `apps/api/Cargo.toml`:

1. In `[dependencies]` and `[dev-dependencies]`, `git = "file:///home/yann/Code/Facile/tronc-rs/.git"`
   becomes `git = "https://github.com/FacileStudio/tronc-rs"`.
2. In `[dependencies]`, `git = "file:///home/yann/Code/Facile/porte-rs"` becomes
   `git = "https://github.com/FacileStudio/porte-rs"`, at a porte-rs tag whose own `tronc`
   dependency is the HTTPS URL at `v0.3.0` or later.
3. Delete the `[patch."file:///home/yann/Code/Facile/tronc-rs"]` section. It exists only because
   porte-rs `v0.4.0` depends on tronc `v0.2.0`, and two tronc versions would make
   `tronc::Error` two different types.
4. Run `cargo update -p tronc -p porte` and commit `Cargo.lock`.

Then register the app: `Registre/seed.yaml` entry (plan 7.8), the Dokploy environment through
the `dokploy-env-safe` flow, `deploy-verify` with `VERIFY_URLS=https://feuilles.facile.studio`,
the suite card, the vitrine entry, a Sonde monitor on `/health`.
