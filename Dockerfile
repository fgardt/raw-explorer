# Build env with cargo-chef and wasm target
FROM rust:slim AS build_env
WORKDIR /work
RUN cargo install --locked cargo-chef
RUN rustup target add wasm32-unknown-unknown

FROM build_env AS planner
COPY . .
RUN cargo chef prepare

FROM build_env AS builder
# Leptos build deps
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl npm libc-dev binaryen
RUN npm install -g sass
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/leptos-rs/cargo-leptos/releases/latest/download/cargo-leptos-installer.sh | sh
# Leptos build env variables
ENV LEPTOS_OUTPUT_NAME=raw-explorer \
    LEPTOS_SITE_ROOT=target/site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    LEPTOS_RELOAD_PORT=3001 \
    LEPTOS_LIB_DIR=. \
    LEPTOS_BIN_DIR=. \
    LEPTOS_JS_MINIFY=true \
    LEPTOS_HASH_FILES=true \
    LEPTOS_HASH_FILE_NAME=hash.txt
# Build dependencies for docker layer caching
COPY --from=planner /work/recipe.json recipe.json
# Frontend deps
RUN cargo chef cook --no-default-features --features hydrate --profile wasm-release --target wasm32-unknown-unknown --package raw-explorer --target-dir target/front
# Backend deps
RUN cargo chef cook --no-default-features --features ssr --release --package raw-explorer --bin raw-explorer

# Build the app
COPY . .
RUN cargo leptos build --release -vv

# Final image
FROM debian:stable-slim AS runner
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /work/target/release/raw-explorer /app/
COPY --from=builder /work/target/release/hash.txt /app/
COPY --from=builder /work/target/site /app/site
COPY --from=builder /work/Cargo.toml /app/

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ROOT=./site \
    LEPTOS_HASH_FILES=true \
    LEPTOS_HASH_FILE_NAME=hash.txt

EXPOSE 3000

ENTRYPOINT ["./raw-explorer"]
