FROM rust:latest AS chef
ARG DEBIAN_FRONTEND=noninteractive

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

WORKDIR /app

FROM chef AS planner
COPY crates/entity/Cargo.toml ./crates/entity/Cargo.toml
COPY crates/rss-common/Cargo.toml ./crates/rss-common/Cargo.toml
COPY crates/fetcher/Cargo.toml ./crates/fetcher/Cargo.toml
COPY crates/api/Cargo.toml ./crates/api/Cargo.toml
COPY Cargo.* ./
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get install libssl-dev -y
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY crates/rss-common/src crates/rss-common/src
COPY crates/entity/src crates/entity/src
COPY crates/fetcher/src crates/fetcher/src
COPY crates/api/src/ crates/api/src/
RUN touch crates/api/src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl --all

FROM alpine AS api
LABEL maintainer=eric@pedr0.net
RUN addgroup -S rss-aggregator && adduser -S rss-aggregator -G rss-aggregator

RUN apk --no-cache add curl tzdata # Needed for the docker health check and fix issue with chrono
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rss-aggregator /usr/local/bin
COPY crates/api/static/ static/

EXPOSE 8080
USER rss-aggregator
ENTRYPOINT ["rss-aggregator"]

FROM alpine as fetcher
LABEL maintainer=eric@pedr0.net
RUN addgroup -S rss-fetcher && adduser -S rss-fetcher -G rss-fetcher

RUN apk --no-cache add curl tzdata # Needed for the docker health check and fix issue with chrono
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/fetcher /usr/local/bin

EXPOSE 8080
USER rss-fetcher
ENTRYPOINT ["fetcher"]
