FROM rust:latest AS chef
ARG DEBIAN_FRONTEND=noninteractive

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
RUN apt update && apt install -y musl-tools musl-dev lld clang git cmake libssl-dev zlib1g-dev gcc g++ file bsdmainutils
RUN update-ca-certificates

# Let's install mold for the fun
RUN git clone https://github.com/rui314/mold.git
WORKDIR mold
RUN git checkout v1.3.1
RUN make -j$(nproc) CXX=clang++
RUN make install PREFIX=/usr

WORKDIR /app

FROM chef AS planner
COPY entity/Cargo.toml ./entity/Cargo.toml
COPY Cargo.* ./
COPY .cargo/ .cargo/
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
COPY .cargo/ .cargo/
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY entity/src entity/src
COPY src/ src/
RUN cargo build --release --target x86_64-unknown-linux-musl --bin rss-aggregator

FROM alpine
LABEL maintainer=eric@pedr0.net
RUN addgroup -S rss-aggregator && adduser -S rss-aggregator -G rss-aggregator

RUN apk --no-cache add curl # Needed for the docker health check

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rss-aggregator /usr/local/bin
COPY static/ static/

EXPOSE 8080
USER rss-aggregator
ENTRYPOINT ["rss-aggregator"]