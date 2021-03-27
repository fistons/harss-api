FROM rust:latest as planner
WORKDIR app
RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust:latest as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:latest as builder
RUN cargo install diesel_cli --no-default-features --features sqlite
WORKDIR app
COPY . .
RUN diesel migration run
COPY --from=cacher /app/target target
RUN cargo build --release --bin rss-aggregator

FROM debian:stable-slim as runtime
RUN apt-get update && apt-get install libssl-dev sqlite3 -y
WORKDIR app
COPY --from=builder /app/target/release/rss-aggregator /usr/local/bin
COPY --from=builder /app/test.db .
COPY static/ static/

ENTRYPOINT ["rss-aggregator"]