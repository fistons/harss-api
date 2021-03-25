FROM rust:latest as cargo-build

WORKDIR /usr/src/

# Build the dependencies. We do this in a seprate cargo build process to avoid redoing it for each build
RUN cargo new --bin rss-aggregator
WORKDIR /usr/src/rss-aggregator
COPY ./Cargo.toml ./Cargo.toml 
COPY ./Cargo.lock ./Cargo.lock
RUN cargo build --release

# Remove temporary files. Not sure it's useful, but it's cost nothing
RUN rm -f target/release/deps/rss-aggregator* 
RUN rm src/*.rs 

# Do the real build this time
COPY ./src ./src
RUN cargo build --release


FROM debian:stable-slim
LABEL maintainer="Eric <eric@pedr0.net>"

RUN apt-get update && apt-get install libssl-dev -y
COPY --from=cargo-build /usr/src/rss-aggregator/target/release/rss-aggregator /usr/local/bin/rss-aggregator
COPY ./static ./static

EXPOSE 8080
ENTRYPOINT ["rss-aggregator"]
