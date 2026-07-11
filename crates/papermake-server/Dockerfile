# 1.89+ required by locked deps (time 0.3.53, enum-ordinalize 4.4.1, …)
ARG RUST_VERSION=1.89.0
ARG PACKAGE_NAME=papermake-server
FROM rust:${RUST_VERSION}-slim AS build

# Install required build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files and build
RUN --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=crates,target=crates \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release -p papermake-server && \
    cp ./target/release/papermake-server /bin/server

FROM debian:bookworm-slim AS final
ARG UID=10001

# Runtime dependencies. curl is for container healthchecks (GET /health) —
# without it, orchestrators running a curl-based check report the container
# unhealthy indefinitely while it serves fine.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

# Bundle the repo fonts: rendering fails without fonts, and image consumers
# have no repo checkout to bind-mount them from.
COPY fonts /fonts

ENV FONTS_DIR=/fonts \
    HOST=0.0.0.0 \
    PORT=3000

USER appuser

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

EXPOSE 3000

# What the container should run when it is started.
CMD ["/bin/server"]
