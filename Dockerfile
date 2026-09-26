# syntax=docker/dockerfile:1

# The web app is embedded into the binary, so it's built first.
FROM node:24-slim AS web
WORKDIR /src/web
RUN npm install --global pnpm@11
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

FROM rust:1.98-slim-trixie AS app
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY migrations migrations
COPY src src
# Only the manifest matters here: the desktop app is a workspace member but isn't built.
COPY desktop desktop
COPY --from=web /src/web/dist web/dist
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --package feedrsauros \
    && cp target/release/feedrsauros /usr/local/bin/feedrsauros

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --uid 10001 --home-dir /data feedrsauros \
    && mkdir /data && chown feedrsauros /data
COPY --from=app /usr/local/bin/feedrsauros /usr/local/bin/feedrsauros

USER feedrsauros
ENV FEEDRSAUROS_DB=/data/feedrsauros.db \
    FEEDRSAUROS_HOST=0.0.0.0 \
    FEEDRSAUROS_PORT=7777 \
    RUST_LOG=feedrsauros=info
VOLUME /data
EXPOSE 7777
ENTRYPOINT ["feedrsauros"]
CMD ["serve"]
