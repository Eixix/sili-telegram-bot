FROM rust:1.90-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config ./config
COPY resources/static ./resources/static
COPY tests/test_data ./tests/test_data
RUN cargo build --locked --release

FROM build AS test
RUN cargo test --locked
ENTRYPOINT ["cargo", "test", "--locked", "--offline"]

FROM debian:bookworm-slim AS prod
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 bot && useradd --uid 1000 --gid bot --no-create-home bot
WORKDIR /bot
COPY --from=build /build/target/release/sili-telegram-bot /usr/local/bin/sili-telegram-bot
COPY config ./config
COPY resources/static ./resources/static
RUN mkdir -p resources/dynamic && chown -R bot:bot resources/dynamic
USER 1000:1000
ENTRYPOINT ["sili-telegram-bot"]
