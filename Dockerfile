FROM rust:latest AS builder
ARG DEBIAN_FRONTEND=noninteractive

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

ENV USER=rss-aggregator
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR app
COPY ./ .
RUN cargo build --target x86_64-unknown-linux-musl --release

### The actual build
FROM alpine
LABEL maintainer=eric@pedr0.net

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group


RUN apk update && apk add libressl-dev libpq-dev ca-certificates curl

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rss-aggregator /usr/local/bin
COPY static/ static/

EXPOSE 8080

USER rss-aggregator:rss-aggregator

ENTRYPOINT ["rss-aggregator"]
