FROM rust:latest AS builder
ARG DEBIAN_FRONTEND=noninteractive

WORKDIR app

### Build the dependencies in a separate layers
# Create a mock source
RUN cargo new rss-aggregator
WORKDIR rss-aggregator
# Copy our Cargo definition
COPY Cargo.lock .
COPY Cargo.toml .
# Build the libs
RUN cargo build --release

### Build our app
# Add our source
ADD .. . 
# Touch our main to change its last access date
RUN touch src/main.rs
# build our source
RUN cargo build --release


### The actual build
FROM debian:buster-slim as runtime
LABEL maintainer=eric@pedr0.net
ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install libssl-dev libpq-dev ca-certificates -y

COPY --from=builder /app/rss-aggregator/target/release/rss-aggregator /usr/local/bin
COPY static static/

EXPOSE 8080

ENTRYPOINT ["rss-aggregator"]