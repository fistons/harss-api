FROM rust:latest as cargo-build

WORKDIR /usr/src/rss

COPY . .

RUN cargo build --release

# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------

FROM debian:stable-slim

RUN apt-get update && apt-get install libssl-dev libsqlite3-dev -y

COPY --from=cargo-build /usr/src/rss/target/release/rss-aggregator /usr/local/bin/rss-aggregator

EXPOSE 8080
CMD ["rss-aggregator"]
