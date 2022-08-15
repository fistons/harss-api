FROM rust:latest AS chef
ARG DEBIAN_FRONTEND=noninteractive

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

WORKDIR /app

FROM chef AS planner
COPY entity/Cargo.toml ./entity/Cargo.toml
COPY fetcher/Cargo.toml ./fetcher/Cargo.toml
COPY Cargo.* ./
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY entity/src entity/src
COPY fetcher/src fetcher/src
COPY src/ src/
RUN touch src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl --bin rss-aggregator

FROM alpine
LABEL maintainer=eric@pedr0.net
RUN addgroup -S rss-aggregator && adduser -S rss-aggregator -G rss-aggregator

RUN apk --no-cache add curl tzdata # Needed for the docker health check and fix issue with chrono
RUN cp /usr/share/zoneinfo/Europe/Paris /etc/localtime

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rss-aggregator /usr/local/bin
COPY static/ static/

EXPOSE 8080
USER rss-aggregator
ENTRYPOINT ["rss-aggregator"]