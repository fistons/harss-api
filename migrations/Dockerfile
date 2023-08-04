FROM rust:latest as builder

RUN update-ca-certificates
RUN cargo install sqlx-cli --no-default-features --features openssl-vendored,postgres

FROM debian:12-slim as migration
LABEL maintainer=eric@pedr0.net
ENV DEBIAN_FRONTEND=noninteractive

COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY *.sql migrations/

RUN adduser migrations
USER migrations