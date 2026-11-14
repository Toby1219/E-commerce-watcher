FROM rust:1.96 AS builder

WORKDIR /api

ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=60

# Step 1: dependency-only build.
# Copy workspace manifests and stub the source files so `cargo build`
# compiles just the dependency tree. Cache mounts keep that work across
# image builds (the mounts are unmounted when the RUN ends, so nothing
# here leaks into later layers).
COPY Cargo.toml Cargo.lock ./
COPY entities/Cargo.toml ./entities/
COPY migration/Cargo.toml ./migration/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/api/target \
    mkdir -p src entities/src migration/src \
    && echo "pub fn placeholder() {}" > src/lib.rs \
    && echo > entities/src/lib.rs \
    && echo > migration/src/lib.rs \
    && cargo build --release

# Step 2: real sources. Dependency artifacts come from the cache mounts;
# only our own crates recompile.
COPY src ./src
COPY entities/src ./entities/src
COPY migration/src ./migration/src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/api/target \
    # bump the mtime of every real .rs file so cargo sees it as newer than the stub build
    find src entities/src migration/src -name '*.rs' -exec touch {} + \
    # compile the real crates (dependencies stay cached)
    && cargo build --release \
    # the cache-mounted target/ is NOT saved in the image, so copy the binary out
    && cp target/release/api /api/api-bin

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /api/api-bin /app/api

EXPOSE 8181

CMD ["./api"]
