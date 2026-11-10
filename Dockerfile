FROM oven/bun:1 AS client-build
WORKDIR /client
COPY apps/client/package.json apps/client/bun.lock* ./
RUN bun install --frozen-lockfile
COPY apps/client/ .
RUN bun run build

FROM rust:1-bookworm AS api-build
WORKDIR /repo/apps/api
COPY apps/api/Cargo.toml apps/api/Cargo.lock apps/api/rust-toolchain.toml ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs && cargo build --release --locked && rm -rf src
COPY apps/api ./
COPY apps/client/package.json /version/package.json
COPY scripts/version.sh /version/version.sh
RUN touch src/main.rs src/lib.rs && APP_VERSION="$(sh /version/version.sh /version/package.json)" cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=api-build /repo/apps/api/target/release/api /api
COPY --from=client-build /client/build /client
ENV CLIENT_DIR=/client
ENV PORT=4000
EXPOSE 4000
USER nonroot:nonroot
ENTRYPOINT ["/api"]
