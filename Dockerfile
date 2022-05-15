FROM rust:latest AS builder
ARG DEBIAN_FRONTEND=noninteractive

WORKDIR app

### Build the dependencies in a separate layers
# Create a mock source
ADD . .
# Build the libs
RUN cargo build --release

### The actual build
FROM debian:buster-slim as runtime
LABEL maintainer=eric@pedr0.net
ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install libssl-dev libpq-dev ca-certificates curl -y

COPY --from=builder /app/target/release/rss-aggregator /usr/local/bin
COPY static/ static/

EXPOSE 8080

ENTRYPOINT ["rss-aggregator"]