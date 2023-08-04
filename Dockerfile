FROM rust:latest AS chef
RUN cargo install cargo-chef
RUN update-ca-certificates

WORKDIR /app

FROM chef AS planner
COPY crates/common/Cargo.toml ./crates/common/Cargo.toml
COPY crates/fetcher/Cargo.toml ./crates/fetcher/Cargo.toml
COPY crates/api/Cargo.toml ./crates/api/Cargo.toml
COPY Cargo.* ./
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get install libssl-dev -y
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY .sqlx .sqlx
COPY crates/common/src crates/common/src
COPY crates/fetcher/src crates/fetcher/src
COPY crates/api/src/ crates/api/src/
RUN touch crates/api/src/main.rs
ENV SQLX_OFFLINE=true
RUN cargo build --release --all

FROM debian:12-slim AS api
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive
RUN adduser rss-fetcher

RUN apt update && apt install -y curl tzdata && rm -rf /var/lib/apt/lists/* 
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY --from=builder /app/target/release/rss-aggregator /usr/local/bin
COPY crates/api/static/ static/

EXPOSE 8080
USER rss-fetcher
ENTRYPOINT ["rss-aggregator"]

FROM debian:12-slim as fetcher
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive
RUN adduser rss-fetcher

RUN apt update && apt install -y curl tzdata && rm -rf /var/lib/apt/lists/* 
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY --from=builder /app/target/release/fetcher /usr/local/bin

EXPOSE 8080
USER rss-fetcher
ENTRYPOINT ["fetcher"]

